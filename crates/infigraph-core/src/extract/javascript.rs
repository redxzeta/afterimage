//! Syntax-only JS/TS import bindings. Callback values are never invocations.
use std::collections::HashMap;
use tree_sitter::Node;

fn text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    std::str::from_utf8(&source[node.byte_range()]).unwrap_or("")
}

/// Encode syntax-proven local or imported callees before raw graph writes.
/// Unknown members must never share a target ID with a bare local symbol.
pub(super) fn imported_call(node: Node<'_>, root: Node<'_>, source: &[u8]) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    let member = (function.kind() == "member_expression")
        .then(|| function.child_by_field_name("property"))
        .flatten();
    let identifier = if member.is_some() {
        function.child_by_field_name("object")?
    } else {
        function
    };
    if member.is_some() && identifier.kind() == "this" {
        let method = text(member?, source);
        let mut parent = node.parent();
        while let Some(scope) = parent {
            // Normal callbacks have their own `this`; arrows inherit it.
            if matches!(
                scope.kind(),
                "function_expression"
                    | "generator_function"
                    | "function_declaration"
                    | "generator_function_declaration"
                    | "class"
            ) {
                break;
            }
            if scope.kind() == "method_definition"
                && scope.parent().is_none_or(|p| p.kind() != "class_body")
            {
                break;
            }
            if scope.kind() == "class_declaration" {
                let class = text(scope.child_by_field_name("name")?, source);
                return Some(format!("{class}::{method}"));
            }
            parent = scope.parent();
        }
        return Some(format!("@unresolved::{method}"));
    }
    if identifier.kind() != "identifier" {
        return member.map(|m| format!("@unresolved::{}", text(m, source)));
    }
    let name = text(identifier, source);
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
    let local_class = member.and_then(|member| {
        let mut cursor = root.walk();
        let found = root.named_children(&mut cursor).find_map(|statement| {
            let class = if statement.kind() == "export_statement" {
                statement.child_by_field_name("declaration")?
            } else {
                statement
            };
            if class.kind() != "class_declaration"
                || text(class.child_by_field_name("name")?, source) != name
            {
                return None;
            }
            let body = class.child_by_field_name("body")?;
            let mut cursor = body.walk();
            let found = body.named_children(&mut cursor).find_map(|method| {
                let mut cursor = method.walk();
                (method.kind() == "method_definition"
                    && method
                        .child_by_field_name("name")
                        .is_some_and(|n| text(n, source) == text(member, source))
                    && method.children(&mut cursor).any(|n| n.kind() == "static"))
                .then(|| ("@local-class".to_owned(), name.to_owned()))
            });
            found
        });
        found
    });
    let binding = imports.get(name).or(local_class.as_ref());
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
                    | "generator_function_declaration"
                    | "function_expression"
                    | "generator_function"
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
    if let Some(member) = member {
        let method = text(member, source);
        return Some(match binding {
            Some((module, exported)) if exported == "*" => format!("{module}::{method}"),
            Some((module, class)) if module == "@local-class" => format!("{class}::{method}"),
            _ => format!("@unresolved::{method}"),
        });
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
        "namespace_import" => {
            let mut cursor = node.walk();
            if let Some(local) = node
                .named_children(&mut cursor)
                .find(|n| n.kind() == "identifier")
            {
                imports.insert(text(local, source).to_owned(), (module.to_owned(), "*".into()));
            }
            return;
        }
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
    if node.kind() == "class_declaration" {
        return node
            .child_by_field_name("name")
            .is_some_and(|n| text(n, source) == name);
    }
    if node.kind() == "variable_declarator" {
        return node
            .child_by_field_name("name")
            .is_some_and(|n| pattern_contains(n, name, source));
    }
    if node.kind() == "for_in_statement"
        && node.child_by_field_name("kind").is_some()
        && node
            .child_by_field_name("left")
            .is_some_and(|n| pattern_contains(n, name, source))
    {
        return true;
    }
    if matches!(
        node.kind(),
        "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "generator_function"
    ) && node
        .child_by_field_name("name")
        .is_some_and(|n| text(n, source) == name)
    {
        return true;
    }
    if matches!(
        node.kind(),
        "formal_parameters" | "required_parameter" | "optional_parameter"
    ) {
        return pattern_contains(node, name, source);
    }
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).any(|child| {
        // A nested declaration contributes its name, never its parameters/body.
        if matches!(
            child.kind(),
            "function_declaration" | "generator_function_declaration"
        ) {
            return child
                .child_by_field_name("name")
                .is_some_and(|n| text(n, source) == name);
        }
        // Don't let declarations in nested sibling scopes shadow this scope.
        if matches!(
            child.kind(),
            "statement_block"
                | "arrow_function"
                | "function_expression"
                | "generator_function"
                | "method_definition"
        ) {
            return false;
        }
        has_binding(child, name, source)
    });
    found
}

/// This deliberately recognizes only direct named exports. Export lists,
/// aliases, default exports and barrels cannot certify a same-name callable.
pub(super) fn direct_named_export(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "export_statement" => {
                let mut cursor = parent.walk();
                let has_default = parent.children(&mut cursor).any(|n| n.kind() == "default");
                return !has_default;
            }
            "lexical_declaration" | "variable_declaration" => node = parent,
            _ => return false,
        }
    }
    false
}
