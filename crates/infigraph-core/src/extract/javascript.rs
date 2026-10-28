//! Syntax-only JS/TS import bindings. Callback values are never invocations.
use std::collections::HashMap;
use tree_sitter::Node;

fn text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source[node.byte_range()]).unwrap_or("")
}

/// Encode a bound imported callee in the unresolved target ID. Keeping the
/// specifier alongside the export name prevents global bare-name guessing.
pub(super) fn imported_call(node: Node<'_>, root: Node<'_>, source: &[u8]) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "identifier" {
        return None;
    }
    let name = text(function, source);
    let mut imports = HashMap::new();
    let mut cursor = root.walk();
    for statement in root
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "import_statement")
    {
        let module = text(statement.child_by_field_name("source")?, source);
        // Type-only imports cannot supply runtime callees.
        if text(statement, source)
            .trim_start()
            .starts_with("import type ")
        {
            continue;
        }
        collect_bindings(statement, module, source, &mut imports);
    }
    let binding = imports.get(name);
    // Lexical locals and parameters shadow imported bindings. Abstain on a
    // shadow instead of treating a same-named local as the imported callable.
    let mut parent = node.parent();
    while let Some(scope) = parent {
        if scope == root {
            break;
        }
        if scope
            .child_by_field_name("parameters")
            .or_else(|| scope.child_by_field_name("parameter"))
            .is_some_and(|n| pattern_contains(n, name, source))
        {
            return Some(format!("@unresolved::{name}"));
        }
        if binding.is_some()
            && matches!(
                scope.kind(),
                "statement_block"
                    | "arrow_function"
                    | "function_declaration"
                    | "function_expression"
                    | "method_definition"
                    | "catch_clause"
                    | "for_statement"
                    | "for_in_statement"
            )
            && has_binding(scope, name, source)
        {
            return Some(format!("@unresolved::{name}"));
        }
        parent = scope.parent();
    }
    let (module, exported) = binding?;
    Some(format!("{module}::{exported}"))
}

fn collect_bindings(
    node: Node<'_>,
    module: &str,
    source: &[u8],
    imports: &mut HashMap<String, (String, String)>,
) {
    match node.kind() {
        "import_specifier" => {
            if text(node, source).trim_start().starts_with("type ") {
                return;
            }
            if let Some(name) = node.child_by_field_name("name") {
                let local = node.child_by_field_name("alias").unwrap_or(name);
                imports.insert(
                    text(local, source).to_owned(),
                    (module.to_owned(), text(name, source).to_owned()),
                );
            }
            return;
        }
        "import_clause" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "identifier" {
                    imports.insert(
                        text(child, source).to_owned(),
                        (module.to_owned(), "default".into()),
                    );
                }
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_bindings(child, module, source, imports);
    }
}

fn pattern_contains(node: Node<'_>, name: &str, source: &[u8]) -> bool {
    if matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier_pattern"
    ) {
        return text(node, source) == name;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|n| pattern_contains(n, name, source));
    found
}

fn has_binding(node: Node<'_>, name: &str, source: &[u8]) -> bool {
    if node
        .child_by_field_name("parameter")
        .is_some_and(|n| pattern_contains(n, name, source))
    {
        return true;
    }
    if matches!(node.kind(), "variable_declarator" | "class_declaration") {
        return node
            .child_by_field_name("name")
            .is_some_and(|n| pattern_contains(n, name, source));
    }
    if node.kind() == "function_declaration"
        && node
            .child_by_field_name("name")
            .is_some_and(|n| text(n, source) == name)
    {
        return true;
    }
    if matches!(
        node.kind(),
        "formal_parameters" | "required_parameter" | "optional_parameter" | "catch_clause"
    ) {
        return pattern_contains(node, name, source);
    }
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).any(|child| {
        // A nested declaration contributes its name, never its parameters/body.
        if child.kind() == "function_declaration" {
            return child
                .child_by_field_name("name")
                .is_some_and(|n| text(n, source) == name);
        }
        // Don't let declarations in nested sibling scopes shadow this scope.
        if matches!(
            child.kind(),
            "statement_block" | "arrow_function" | "function_expression" | "method_definition"
        ) {
            return false;
        }
        has_binding(child, name, source)
    });
    found
}
