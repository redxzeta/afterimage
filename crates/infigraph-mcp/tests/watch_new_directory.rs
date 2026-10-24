use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use infigraph_core::graph::{GraphBackend, KuzuBackend};
use infigraph_core::lang::{LanguagePack, LanguageRegistry, ParserBackend};
use infigraph_core::watch::{watch_project, WatchEvent, WatchEventKind};

fn python_registry() -> impl Fn() -> anyhow::Result<LanguageRegistry> + Send {
    let bundled = infigraph_languages::bundled_registry().unwrap();
    let ParserBackend::TreeSitter { grammar, .. } = &bundled.for_extension(".py").unwrap().backend
    else {
        panic!("Python must use tree-sitter");
    };
    let grammar = grammar.clone();
    move || {
        let mut registry = LanguageRegistry::new();
        registry.register(LanguagePack::new(
            "python",
            vec![".py"],
            grammar.clone(),
            "(function_definition name: (identifier) @func.name) @func.def",
            "",
        )?);
        Ok(registry)
    }
}

fn start_watch(
    root: &Path,
    registry: impl Fn() -> anyhow::Result<LanguageRegistry> + Send + 'static,
) -> (Watcher, mpsc::Receiver<WatchEvent>) {
    let (stop, stop_rx) = mpsc::channel();
    let (event_tx, events) = mpsc::channel();
    let root = root.to_path_buf();
    let watcher = Watcher {
        stop,
        thread: Some(std::thread::spawn(move || {
            watch_project(&root, registry, 50, stop_rx, move |event| {
                let _ = event_tx.send(event);
            })
        })),
    };
    (watcher, events)
}

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
    let (_watcher, events) = start_watch(&root, python_registry());

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
    let outside = fixture.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    let outside_file = outside.join("outside.py");
    let outside_bytes = b"def outside_marker(): pass\n";
    std::fs::write(&outside_file, outside_bytes).unwrap();
    let outside_modified = outside_file.metadata().unwrap().modified().unwrap();
    #[cfg(target_os = "linux")]
    {
        std::os::unix::fs::symlink(&outside, staged.join("external")).unwrap();
        std::os::unix::fs::symlink(".", staged.join("loop")).unwrap();
    }
    let added = root.join("module");
    std::fs::rename(staged, &added).unwrap();
    wait_for(&events, &added.join("direct.py"), WatchEventKind::Modified);
    let mut indexed = names(&root);
    indexed.sort();
    assert_eq!(indexed, ["direct_marker", "nested_marker"]);

    // The new directory itself must be watched, not just its subdirectories.
    let direct = added.join("direct.py");
    #[cfg(target_os = "linux")]
    std::os::unix::fs::symlink(&outside, root.join("direct_link")).unwrap();
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
    assert_eq!(std::fs::read(&outside_file).unwrap(), outside_bytes);
    assert_eq!(
        outside_file.metadata().unwrap().modified().unwrap(),
        outside_modified
    );
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 1);
}

#[cfg(target_os = "linux")]
fn wait_for_watch_registration(root: &Path) {
    use std::os::unix::fs::MetadataExt;
    let inode = root.metadata().unwrap().ino();
    let deadline = Instant::now() + Duration::from_secs(12);
    loop {
        for entry in std::fs::read_dir("/proc/self/fdinfo").unwrap().flatten() {
            let Ok(info) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            if info.lines().any(|line| {
                line.starts_with("inotify wd:")
                    && line.split_whitespace().any(|field| {
                        field
                            .strip_prefix("ino:")
                            .and_then(|value| u64::from_str_radix(value, 16).ok())
                            == Some(inode)
                    })
            }) {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "watcher did not register the fixture root"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn expect_names(root: &Path, expected: &[&str]) {
    expect_names_before(root, expected, Instant::now() + Duration::from_secs(12));
}

#[cfg(target_os = "linux")]
fn expect_names_before(root: &Path, expected: &[&str], deadline: Instant) {
    let mut expected: Vec<_> = expected.iter().map(|name| name.to_string()).collect();
    expected.sort();
    let mut last_observation = String::from("not read yet");
    let mut read_errors = 0;
    loop {
        assert!(
            Instant::now() < deadline,
            "graph freshness deadline: expected {expected:?}, last observation {last_observation}"
        );
        // This is an eventual-freshness observation during active writes. A
        // clean native read error is unsuccessful, never a passing snapshot.
        let snapshot = KuzuBackend::open_read_only(&root.join(".infigraph/graph"))
            .and_then(|reader| reader.get_symbols_for_search());
        assert!(
            Instant::now() <= deadline,
            "graph freshness deadline expired during read"
        );
        match snapshot {
            Ok(rows) => {
                let mut actual: Vec<_> = rows.into_iter().map(|row| row[1].clone()).collect();
                actual.sort();
                if actual == expected {
                    eprintln!("fresh graph snapshot; unsuccessful native reads: {read_errors}");
                    return;
                }
                last_observation = format!("{actual:?}");
            }
            Err(error) => {
                read_errors += 1;
                last_observation = error.to_string();
                if read_errors <= 3 {
                    eprintln!("unsuccessful native read: {error}");
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(target_os = "linux")]
fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fixture git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(target_os = "linux")]
fn return_checkout_restores_graph_within_original_budget() {
    let fixture = tempfile::Builder::new()
        .prefix("infigraph-return-")
        .tempdir()
        .unwrap();
    let root = fixture.path();
    git(root, &["init", "--template=", "-b", "a"]);
    std::fs::write(root.join(".gitignore"), ".infigraph/\n").unwrap();
    std::fs::write(root.join("app.py"), "def branch_a_marker(): pass\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-m", "fixture A"]);
    git(root, &["checkout", "-b", "b"]);
    std::fs::remove_file(root.join("app.py")).unwrap();
    std::fs::create_dir_all(root.join("module/sub")).unwrap();
    std::fs::write(
        root.join("module/sub/new.py"),
        "def branch_b_marker(): pass\n",
    )
    .unwrap();
    git(root, &["add", "-A"]);
    git(root, &["commit", "-m", "fixture B"]);
    git(root, &["checkout", "a"]);
    {
        let mut prism =
            infigraph_core::Infigraph::open(root, infigraph_languages::bundled_registry().unwrap())
                .unwrap();
        prism.init().unwrap();
        prism.index().unwrap();
    }
    let (_watcher, _events) = start_watch(root, infigraph_languages::bundled_registry);
    wait_for_watch_registration(root);
    git(root, &["checkout", "b"]);
    expect_names(root, &["branch_b_marker"]);
    let returning = Instant::now();
    git(root, &["checkout", "a"]);
    expect_names_before(
        root,
        &["branch_a_marker"],
        returning + Duration::from_secs(12),
    );
    eprintln!(
        "return checkout graph fresh after {:?}",
        returning.elapsed()
    );
    std::fs::write(root.join("app.py"), "def working_edit(): pass\n").unwrap();
    expect_names(root, &["working_edit"]);
    std::fs::remove_file(root.join("app.py")).unwrap();
    expect_names(root, &[]);
}

#[test]
#[cfg(target_os = "linux")]
fn hidden_ancestor_allows_source_changes_but_preserves_ignores() {
    let fixture = tempfile::Builder::new()
        .prefix("infigraph-hidden-")
        .tempdir()
        .unwrap();
    let root = fixture.path().join(".private-parent/project");
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init", "--template="]);
    std::fs::write(
        root.join(".gitignore"),
        "ignored.py\nignored_dir/\n*.secret.py\n",
    )
    .unwrap();
    std::fs::write(root.join(".infigraphignore"), "custom_private.py\n").unwrap();
    std::fs::write(root.join("app.py"), "def original_marker(): pass\n").unwrap();
    let registry = python_registry();
    {
        let mut prism = infigraph_core::Infigraph::open(&root, registry().unwrap()).unwrap();
        prism.init().unwrap();
        prism.index().unwrap();
    }
    let (_watcher, _events) = start_watch(&root, registry);
    wait_for_watch_registration(&root);
    std::fs::write(root.join("app.py"), "def edited_marker(): pass\n").unwrap();
    expect_names(&root, &["edited_marker"]);

    let excluded = [
        ".git/leak.py",
        ".infigraph/leak.py",
        "build/leak.py",
        ".private/leak.py",
        "ignored.py",
        "ignored_dir/leak.py",
        "custom_private.py",
    ];
    for relative in excluded {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "def excluded_marker(): pass\n").unwrap();
    }
    let staged = fixture.path().join("staged");
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(staged.join(".gitignore"), "!keep.secret.py\n").unwrap();
    std::fs::write(staged.join("keep.secret.py"), "def kept_marker(): pass\n").unwrap();
    std::fs::write(
        staged.join("drop.secret.py"),
        "def excluded_marker(): pass\n",
    )
    .unwrap();
    std::fs::rename(staged, root.join("new_module")).unwrap();
    expect_names(&root, &["edited_marker", "kept_marker"]);
    for relative in excluded {
        std::fs::write(root.join(relative), "def later_excluded_marker(): pass\n").unwrap();
    }
    std::fs::write(root.join("app.py"), "def later_edit(): pass\n").unwrap();
    expect_names(&root, &["later_edit", "kept_marker"]);
    std::fs::remove_file(root.join("app.py")).unwrap();
    expect_names(&root, &["kept_marker"]);
    std::fs::remove_dir_all(root.join("new_module")).unwrap();
    expect_names(&root, &[]);
}

#[test]
#[cfg(target_os = "linux")]
fn changed_ignore_rules_exclude_a_file_already_queued_for_indexing() {
    let fixture = tempfile::Builder::new()
        .prefix("infigraph-queued-ignore-")
        .tempdir()
        .unwrap();
    let root = fixture.path();
    git(root, &["init", "--template="]);
    std::fs::write(root.join("app.py"), "def stable_marker(): pass\n").unwrap();
    let registry = python_registry();
    {
        let mut prism = infigraph_core::Infigraph::open(root, registry().unwrap()).unwrap();
        prism.init().unwrap();
        prism.index().unwrap();
    }
    let control = root.join("pause.txt");
    std::fs::write(&control, "control").unwrap();
    let (paused_tx, paused_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let (stop, stop_rx) = mpsc::channel();
    let watched_root = root.to_path_buf();
    let _watcher = Watcher {
        stop,
        thread: Some(std::thread::spawn(move || {
            watch_project(&watched_root, registry, 50, stop_rx, move |event| {
                if event.path == control && event.kind == WatchEventKind::Removed {
                    paused_tx.send(()).unwrap();
                    resume_rx.recv_timeout(Duration::from_secs(8)).unwrap();
                }
            })
        })),
    };
    wait_for_watch_registration(root);
    // Same-directory inotify events are ordered: the source event is handled
    // before this remove callback pauses the watcher, but isn't flushed yet.
    std::fs::write(
        root.join("queued.py"),
        "def queued_private_marker(): pass\n",
    )
    .unwrap();
    std::fs::remove_file(root.join("pause.txt")).unwrap();
    paused_rx.recv_timeout(Duration::from_secs(12)).unwrap();
    assert_eq!(names(root), ["stable_marker"]);
    std::fs::write(root.join(".gitignore"), "queued.py\n").unwrap();
    // Expire the ordinary batch window while the rule-change event is still
    // unread. Flush must inspect the current rules before opening the graph.
    std::thread::sleep(Duration::from_millis(1200));
    resume_tx.send(()).unwrap();
    std::fs::write(root.join("app.py"), "def after_rules_marker(): pass\n").unwrap();
    expect_names(root, &["after_rules_marker"]);
}
