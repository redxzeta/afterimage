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
        if type_only_import(statement) {
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
                    | "class"
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

// `type` is an unnamed grammar token here, not necessarily followed by a
// space (newlines and comments are valid between import tokens).
fn type_only_import(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|n| n.kind() == "type");
    found
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
                imports.insert(
                    text(local, source).to_owned(),
                    (module.to_owned(), "*".into()),
                );
            }
            return;
        }
        "import_specifier" => {
            if type_only_import(node) {
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
    if matches!(node.kind(), "class_declaration" | "class") {
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

/// Effect v3's generator factories return a callable, including when trailing
/// inline transforms map the produced Effect. Only direct runtime imports and
/// inline generators certify this narrow case; arbitrary call-valued variables
/// (including invocations of the resulting callable) remain variables.
pub(super) fn effect_function<'a>(node: Node<'a>, source: &[u8]) -> Option<Node<'a>> {
    if node.kind() != "variable_declarator" {
        return None;
    }
    let value = node.child_by_field_name("value")?;
    if value.kind() != "call_expression" {
        return None;
    }
    let arguments = value.child_by_field_name("arguments")?;
    let mut cursor = arguments.walk();
    let mut args = arguments
        .named_children(&mut cursor)
        .filter(|n| n.kind() != "comment");
    let generator = args.next()?;
    if generator.kind() != "generator_function"
        || !args.all(|n| matches!(n.kind(), "arrow_function" | "function_expression"))
    {
        return None;
    }
    let function = value.child_by_field_name("function")?;
    let (factory, expected) = if function.kind() == "call_expression" {
        let names = function.child_by_field_name("arguments")?;
        let mut cursor = names.walk();
        let names: Vec<_> = names
            .named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .collect();
        if names.len() != 1 || names[0].kind() != "string" {
            return None;
        }
        (function.child_by_field_name("function")?, "fn")
    } else {
        (function, "fnUntraced")
    };
    if factory.kind() != "member_expression"
        || text(factory.child_by_field_name("property")?, source) != expected
    {
        return None;
    }
    let receiver = factory.child_by_field_name("object")?;
    if receiver.kind() != "identifier" {
        return None;
    }
    let name = text(receiver, source);
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut imports = HashMap::new();
    let mut cursor = root.walk();
    for statement in root
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "import_statement")
    {
        if type_only_import(statement) {
            continue;
        }
        let module = text(statement.child_by_field_name("source")?, source);
        collect_bindings(statement, module, source, &mut imports);
    }
    let (module, exported) = imports.get(name)?;
    let module = module.trim_matches(['\'', '"']);
    if !((module == "effect/Effect" && exported == "*")
        || (module == "effect" && exported == "Effect"))
    {
        return None;
    }
    // Symbol IDs currently omit lexical scope. Abstain on duplicate target
    // names and on any local receiver binding in the file, including hoisted
    // `var`s. This intentionally also excludes harmless sibling shadows.
    let target = text(node.child_by_field_name("name")?, source);
    if conflicting_effect_binding(root, node, name, target, source) {
        return None;
    }
    Some(generator)
}

fn conflicting_effect_binding(
    node: Node<'_>,
    definition: Node<'_>,
    receiver: &str,
    target: &str,
    source: &[u8],
) -> bool {
    if node.kind() == "import_statement" {
        return false;
    }
    if matches!(
        node.kind(),
        "variable_declarator"
            | "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "class"
            | "function_expression"
            | "generator_function"
            | "interface_declaration"
            | "enum_declaration"
            | "type_alias_declaration"
    ) && node.child_by_field_name("name").is_some_and(|n| {
        text(n, source) == receiver
            || binding_pattern_contains(n, receiver, source)
            || (node != definition
                && (text(n, source) == target || binding_pattern_contains(n, target, source)))
    }) {
        return true;
    }
    if node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"))
        .is_some_and(|n| binding_pattern_contains(n, receiver, source))
        || (node.kind() == "for_in_statement"
            && node.child_by_field_name("kind").is_some()
            && node
                .child_by_field_name("left")
                .is_some_and(|n| binding_pattern_contains(n, receiver, source)))
    {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|n| conflicting_effect_binding(n, definition, receiver, target, source));
    found
}

// Inspect binding positions only: type annotations, object keys and default
// value expressions can mention Effect without declaring a local Effect.
fn binding_pattern_contains(node: Node<'_>, name: &str, source: &[u8]) -> bool {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => text(node, source) == name,
        "required_parameter" | "optional_parameter" => node
            .child_by_field_name("pattern")
            .or_else(|| node.child_by_field_name("name"))
            .is_some_and(|n| binding_pattern_contains(n, name, source)),
        "assignment_pattern" | "object_assignment_pattern" => node
            .child_by_field_name("left")
            .is_some_and(|n| binding_pattern_contains(n, name, source)),
        "pair_pattern" => node
            .child_by_field_name("value")
            .is_some_and(|n| binding_pattern_contains(n, name, source)),
        "formal_parameters" | "object_pattern" | "array_pattern" | "rest_pattern" => {
            let mut cursor = node.walk();
            let found = node
                .named_children(&mut cursor)
                .any(|n| binding_pattern_contains(n, name, source));
            found
        }
        _ => false,
    }
}

// `var` declarations in nested blocks still bind throughout their function.
// Inspect those blocks while excluding nested functions with separate scopes.
fn hoisted_jsx_binding(node: Node<'_>, name: &str, source: &[u8]) -> bool {
    if node.kind() == "variable_declaration" {
        let mut cursor = node.walk();
        if node.named_children(&mut cursor).any(|child| {
            child
                .child_by_field_name("name")
                .is_some_and(|pattern| binding_pattern_contains(pattern, name, source))
        }) {
            return true;
        }
    }
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).any(|child| {
        !matches!(
            child.kind(),
            "arrow_function"
                | "function_declaration"
                | "generator_function_declaration"
                | "function_expression"
                | "generator_function"
                | "method_definition"
                | "class"
                | "class_declaration"
        ) && hoisted_jsx_binding(child, name, source)
    });
    found
}

/// JSX denotes a component value reference, never a lexical invocation.
/// Only identifier tags with a runtime lexical binding are eligible. Namespace,
/// member, intrinsic and unknown tags deliberately remain unresolved.
pub(super) fn jsx_relations(
    root: Node<'_>,
    source: &[u8],
    file: &str,
    symbols: &[crate::model::Symbol],
) -> Vec<crate::model::Relation> {
    use crate::model::{Relation, RelationKind, Span, SymbolKind};
    fn visit<'a>(node: Node<'a>, nodes: &mut Vec<Node<'a>>) {
        if matches!(
            node.kind(),
            "jsx_opening_element" | "jsx_self_closing_element"
        ) {
            nodes.push(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            visit(child, nodes);
        }
    }
    let mut nodes = Vec::new();
    visit(root, &mut nodes);
    let mut imports = HashMap::new();
    let mut cursor = root.walk();
    for statement in root
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "import_statement")
    {
        if !type_only_import(statement) {
            if let Some(module) = statement.child_by_field_name("source") {
                collect_bindings(statement, text(module, source), source, &mut imports);
            }
        }
    }
    let mut relations = Vec::new();
    for node in nodes {
        let Some(tag) = node
            .child_by_field_name("name")
            .filter(|n| n.kind() == "identifier")
        else {
            continue;
        };
        let name = text(tag, source);
        if !name.chars().next().is_some_and(char::is_uppercase) {
            continue;
        }
        let mut scope = node.parent();
        let mut shadowed = false;
        while let Some(parent) = scope.filter(|n| *n != root) {
            if parent
                .child_by_field_name("parameters")
                .or_else(|| parent.child_by_field_name("parameter"))
                .is_some_and(|n| pattern_contains(n, name, source))
                || (matches!(
                    parent.kind(),
                    "statement_block"
                        | "class"
                        | "arrow_function"
                        | "function_declaration"
                        | "generator_function_declaration"
                        | "function_expression"
                        | "generator_function"
                        | "method_definition"
                        | "catch_clause"
                        | "for_statement"
                        | "for_in_statement"
                ) && has_binding(parent, name, source))
                || (matches!(
                    parent.kind(),
                    "arrow_function"
                        | "function_declaration"
                        | "generator_function_declaration"
                        | "function_expression"
                        | "generator_function"
                        | "method_definition"
                ) && hoisted_jsx_binding(parent, name, source))
            {
                shadowed = true;
                break;
            }
            scope = parent.parent();
        }
        if shadowed || (imports.contains_key(name) && has_binding(root, name, source)) {
            continue;
        }
        let target = if let Some((module, exported)) = imports.get(name) {
            if exported == "*" || exported == "default" {
                continue;
            }
            format!("{file}::{module}::{exported}")
        } else {
            // Only direct module declarations can be used without a lexical
            // binding graph. Nested locals, aliases and duplicate IDs abstain.
            let mut cursor = root.walk();
            let declared = root.named_children(&mut cursor).any(|statement| {
                let declaration = if statement.kind() == "export_statement" {
                    statement
                        .child_by_field_name("declaration")
                        .unwrap_or(statement)
                } else {
                    statement
                };
                if declaration.kind() == "function_declaration" {
                    return declaration
                        .child_by_field_name("name")
                        .is_some_and(|n| text(n, source) == name);
                }
                if !matches!(
                    declaration.kind(),
                    "lexical_declaration" | "variable_declaration"
                ) {
                    return false;
                }
                let mut cursor = declaration.walk();
                let declared = declaration.named_children(&mut cursor).any(|n| {
                    n.kind() == "variable_declarator"
                        && n.child_by_field_name("name")
                            .is_some_and(|n| text(n, source) == name)
                });
                declared
            });
            if !declared {
                continue;
            }
            format!("{file}::{name}")
        };
        let start = (
            node.start_position().row as u32 + 1,
            node.start_position().column as u32,
        );
        let end = (
            node.end_position().row as u32 + 1,
            node.end_position().column as u32,
        );
        let Some(owner) = symbols
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    SymbolKind::Function
                        | SymbolKind::Method
                        | SymbolKind::Test
                        | SymbolKind::Variable
                ) && (s.span.start_line, s.span.start_col) <= start
                    && (s.span.end_line, s.span.end_col) >= end
            })
            .max_by_key(|s| {
                (
                    (s.span.start_line, s.span.start_col),
                    std::cmp::Reverse((s.span.end_line, s.span.end_col)),
                )
            })
        else {
            continue;
        };
        relations.push(Relation {
            source_id: owner.id.clone(),
            target_id: target,
            kind: RelationKind::Custom("JSX_USES".into()),
            span: Some(Span {
                file: file.into(),
                start_line: start.0,
                start_col: start.1,
                end_line: end.0,
                end_col: end.1,
            }),
            receiver: None,
        });
    }
    relations
}
