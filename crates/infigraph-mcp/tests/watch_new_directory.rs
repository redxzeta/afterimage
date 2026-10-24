use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use infigraph_core::graph::{GraphBackend, KuzuBackend};
use infigraph_core::lang::{LanguagePack, LanguageRegistry, ParserBackend};
use infigraph_core::watch::{watch_project, WatchEvent, WatchEventKind};

struct Watcher {
    stop: mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<anyhow::Result<()>>>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            thread.join().expect("watch thread panicked").unwrap();
        }
    }
}

fn wait_for(events: &mpsc::Receiver<WatchEvent>, path: &Path, kind: WatchEventKind) {
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let event = events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("watcher did not process the expected change");
        if event.path == path && event.kind == kind {
            return;
        }
    }
}

fn names(root: &Path) -> Vec<String> {
    // Events are emitted before the batch connection closes. A read can
    // correctly report a transient checkpoint conflict during that close.
    let deadline = Instant::now() + Duration::from_secs(3);
    let reader = loop {
        match KuzuBackend::open_read_only(&root.join(".infigraph/graph")) {
            Ok(reader) => break reader,
            Err(error)
                if error.to_string().contains("checkpoint is in progress")
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("graph read failed: {error}"),
        }
    };
    reader
        .get_symbols_for_search()
        .unwrap()
        .into_iter()
        .map(|row| row[1].clone())
        .collect()
}

#[test]
fn populated_new_directory_is_indexed_and_remains_watched() {
    // A visible ancestor isolates new-directory handling from the known
    // absolute hidden-path filter limitation. Never use a registered project.
    let fixture = tempfile::Builder::new()
        .prefix("infigraph-new-dir-")
        .tempdir()
        .unwrap();
    let root = fixture.path().join("project");
    std::fs::create_dir(&root).unwrap();
    // Reuse the bundled Python grammar with a small query, so every change
    // batch need not compile every bundled language just to index two functions.
    let bundled = infigraph_languages::bundled_registry().unwrap();
    let ParserBackend::TreeSitter { grammar, .. } = &bundled.for_extension(".py").unwrap().backend
    else {
        panic!("Python must use tree-sitter");
    };
    let grammar = grammar.clone();
    let registry = move || {
        let mut registry = LanguageRegistry::new();
        registry.register(LanguagePack::new(
            "python",
            vec![".py"],
            grammar.clone(),
            "(function_definition name: (identifier) @func.name) @func.def",
            "",
        )?);
        Ok(registry)
    };
    let (stop, stop_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();
    let watched_root = root.clone();
    let _watcher = Watcher {
        stop,
        thread: Some(std::thread::spawn(move || {
            watch_project(&watched_root, registry, 50, stop_rx, move |event| {
                let _ = event_tx.send(event);
            })
        })),
    };

    // Observe an actual root event instead of assuming a sleep means ready.
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        let control = root.join("ready.txt");
        std::fs::write(&control, "ready").unwrap();
        std::fs::remove_file(&control).unwrap();
        if events.recv_timeout(Duration::from_millis(500)).is_ok() {
            break;
        }
        assert!(Instant::now() < deadline, "watcher did not arm");
    }

    // Moving a populated tree produces a directory event without subsequent
    // per-file creates. This models a fast checkout without timing a race.
    let staged = fixture.path().join("staged");
    for dir in ["sub", "build", ".hidden"] {
        std::fs::create_dir_all(staged.join(dir)).unwrap();
    }
    std::fs::write(staged.join("direct.py"), "def direct_marker(): pass\n").unwrap();
    std::fs::write(staged.join("sub/deep.py"), "def nested_marker(): pass\n").unwrap();
    for path in ["build/ignored.py", ".hidden/ignored.py", "ignored.txt"] {
        std::fs::write(staged.join(path), "def excluded_marker(): pass\n").unwrap();
    }
    let added = root.join("module");
    std::fs::rename(staged, &added).unwrap();
    wait_for(&events, &added.join("direct.py"), WatchEventKind::Modified);
    let mut indexed = names(&root);
    indexed.sort();
    assert_eq!(indexed, ["direct_marker", "nested_marker"]);

    // The new directory itself must be watched, not just its subdirectories.
    let direct = added.join("direct.py");
    std::fs::write(&direct, "def edited_marker(): pass\n").unwrap();
    wait_for(&events, &direct, WatchEventKind::Modified);
    let indexed = names(&root);
    assert!(indexed.contains(&"edited_marker".to_string()));
    assert!(!indexed.contains(&"direct_marker".to_string()));

    let nested: PathBuf = added.join("sub/deep.py");
    std::fs::remove_file(&nested).unwrap();
    wait_for(&events, &nested, WatchEventKind::Removed);
    assert!(!names(&root).contains(&"nested_marker".to_string()));
    std::fs::remove_dir_all(&added).unwrap();
    wait_for(&events, &added, WatchEventKind::Removed);
    assert!(names(&root).is_empty());
}
