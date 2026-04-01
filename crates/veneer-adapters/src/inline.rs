//! Inline JSX parser for documentation code blocks.
//!
//! Parses inline JSX snippets like `<Button variant="default">Click me</Button>`
//! to extract component name, props, and children.
//!
//! Uses oxc AST parsing instead of regex for correct handling of nested
//! components, arrow functions in props, and other complex JSX patterns.

use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Expression, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXElementName,
    JSXExpression, Statement,
};
use oxc_span::{GetSpan as _, SourceType};

/// Parsed inline JSX element.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineJsx {
    /// Component name (e.g., "Button")
    pub component: String,

    /// Props as key-value pairs
    pub props: HashMap<String, PropValue>,

    /// Children content (text or nested JSX as string)
    pub children: Option<String>,

    /// Whether self-closing
    pub self_closing: bool,
}

/// A prop value from JSX.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// String literal: variant="default"
    String(String),
    /// Boolean (presence): disabled
    Boolean(bool),
    /// Expression: onClick={() => {}}
    Expression(String),
}

impl PropValue {
    /// Get as string if it's a string value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PropValue::String(s) => Some(s),
            _ => None,
        }
    }
}

/// Parse inline JSX source code.
///
/// Returns the first top-level JSX element found.
pub fn parse_inline_jsx(source: &str) -> Option<InlineJsx> {
    parse_inline_jsx_all(source).into_iter().next()
}

/// Parse inline JSX source code, returning ALL top-level JSX elements.
///
/// Handles multi-element code blocks with comments between elements,
/// which is the common pattern in rafters component documentation.
/// Each JSX element in the source becomes a separate `InlineJsx` entry.
///
/// Strategy: first try parsing as-is. If the parser panics (which happens
/// with adjacent JSX elements like `<A/><B/>`), wrap in a fragment and
/// extract children individually.
pub fn parse_inline_jsx_all(source: &str) -> Vec<InlineJsx> {
    let trimmed = source.trim();

    // First attempt: parse directly (works for single elements)
    let results = try_parse_jsx_elements(trimmed);
    if !results.is_empty() {
        return results;
    }

    // Second attempt: strip comments and wrap in a fragment.
    // Adjacent JSX elements cause oxc_parser to panic because they're
    // not valid JS without semicolons. Wrapping in <>..</> makes them
    // valid JSX children.
    let stripped = strip_js_line_comments(trimmed);
    let wrapped = format!("<>\n{}\n</>", stripped);

    try_parse_jsx_elements(&wrapped)
}

/// Strip single-line JS comments (// ...) from source.
/// Preserves structure so spans remain usable for simple extraction.
fn strip_js_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Attempt to parse JSX source and extract all top-level elements.
fn try_parse_jsx_elements(source: &str) -> Vec<InlineJsx> {
    let allocator = Allocator::default();
    let source_type = SourceType::jsx();
    let ret = oxc_parser::Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        return Vec::new();
    }

    let mut results = Vec::new();

    for stmt in &ret.program.body {
        if let Statement::ExpressionStatement(expr_stmt) = stmt {
            match &expr_stmt.expression {
                Expression::JSXElement(el) => {
                    results.push(extract_jsx_element(el, source));
                }
                Expression::JSXFragment(frag) => {
                    // Fragment: extract each child element separately
                    for child in &frag.children {
                        if let JSXChild::Element(el) = child {
                            results.push(extract_jsx_element(el, source));
                        }
                    }
                }
                _ => continue,
            }
        }
    }

    results
}

/// Extract component name from a JSXElementName.
fn extract_element_name(name: &JSXElementName<'_>) -> String {
    match name {
        JSXElementName::Identifier(ident) => ident.name.to_string(),
        JSXElementName::IdentifierReference(ident) => ident.name.to_string(),
        JSXElementName::NamespacedName(ns) => {
            format!("{}:{}", ns.namespace.name, ns.name.name)
        }
        JSXElementName::MemberExpression(member) => format_member_expression(member),
        JSXElementName::ThisExpression(_) => "this".to_string(),
    }
}

/// Format a JSX member expression into a dotted name.
fn format_member_expression(member: &oxc_ast::ast::JSXMemberExpression<'_>) -> String {
    let object = match &member.object {
        oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(ident) => {
            ident.name.to_string()
        }
        oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(inner) => {
            format_member_expression(inner)
        }
        oxc_ast::ast::JSXMemberExpressionObject::ThisExpression(_) => "this".to_string(),
    };
    format!("{}.{}", object, member.property.name)
}

/// Extract an InlineJsx from a parsed JSXElement.
fn extract_jsx_element(el: &oxc_ast::ast::JSXElement<'_>, source: &str) -> InlineJsx {
    let component = extract_element_name(&el.opening_element.name);
    let self_closing = el.closing_element.is_none();
    let props = extract_props(&el.opening_element.attributes, source);
    let children = extract_children(&el.children, source);

    InlineJsx {
        component,
        props,
        children,
        self_closing,
    }
}

/// Extract props from JSX attributes.
fn extract_props(
    attributes: &oxc_allocator::Vec<'_, JSXAttributeItem<'_>>,
    source: &str,
) -> HashMap<String, PropValue> {
    let mut props = HashMap::new();

    for attr_item in attributes {
        if let JSXAttributeItem::Attribute(attr) = attr_item {
            let name = match &attr.name {
                JSXAttributeName::Identifier(ident) => ident.name.to_string(),
                JSXAttributeName::NamespacedName(ns) => {
                    format!("{}:{}", ns.namespace.name, ns.name.name)
                }
            };

            let value = match &attr.value {
                None => PropValue::Boolean(true),
                Some(JSXAttributeValue::StringLiteral(lit)) => {
                    PropValue::String(lit.value.to_string())
                }
                Some(JSXAttributeValue::ExpressionContainer(container)) => {
                    match &container.expression {
                        JSXExpression::EmptyExpression(_) => PropValue::Expression(String::new()),
                        expr => {
                            let span = expr.span();
                            let expr_text = &source[span.start as usize..span.end as usize];
                            PropValue::Expression(expr_text.to_string())
                        }
                    }
                }
                Some(JSXAttributeValue::Element(_) | JSXAttributeValue::Fragment(_)) => {
                    // JSX element or fragment as attribute value -- extract as expression text.
                    let span = match &attr.value {
                        Some(JSXAttributeValue::Element(el)) => el.span,
                        Some(JSXAttributeValue::Fragment(frag)) => frag.span,
                        _ => continue,
                    };
                    let text = &source[span.start as usize..span.end as usize];
                    PropValue::Expression(text.to_string())
                }
            };

            props.insert(name, value);
        }
        // SpreadAttribute items are skipped for now -- they don't map to key-value props.
    }

    props
}

/// Extract children text content from JSX children.
///
/// For simple text children, returns the trimmed text.
/// For nested elements or expressions, returns the raw source text of all children combined.
fn extract_children(
    children: &oxc_allocator::Vec<'_, JSXChild<'_>>,
    source: &str,
) -> Option<String> {
    if children.is_empty() {
        return None;
    }

    // If there is exactly one text child, return it trimmed (matching old behavior).
    if children.len() == 1 {
        if let JSXChild::Text(text) = &children[0] {
            let trimmed = text.value.trim();
            if trimmed.is_empty() {
                return None;
            }
            return Some(trimmed.to_string());
        }
    }

    // For mixed or complex children, extract the full source range.
    let first_span = children[0].span();
    let last_span = children[children.len() - 1].span();

    let start = first_span.start as usize;
    let end = last_span.end as usize;
    let text = source[start..end].trim();

    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Convert parsed inline JSX to a Web Component custom element tag.
pub fn to_custom_element(jsx: &InlineJsx, tag_name: &str) -> String {
    let mut attrs = Vec::new();

    for (key, value) in &jsx.props {
        match value {
            PropValue::String(s) => {
                attrs.push(format!(r#"{}="{}""#, key, html_escape(s)));
            }
            PropValue::Boolean(true) => {
                attrs.push(key.clone());
            }
            PropValue::Boolean(false) => {}
            PropValue::Expression(_) => {
                // Skip expressions for static preview
            }
        }
    }

    let attrs_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" {}", attrs.join(" "))
    };

    match &jsx.children {
        Some(children) => {
            format!("<{tag_name}{attrs_str}>{children}</{tag_name}>")
        }
        None => {
            format!("<{tag_name}{attrs_str}></{tag_name}>")
        }
    }
}

/// Escape HTML special characters including single quotes for XSS prevention.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_self_closing() {
        let jsx = parse_inline_jsx(r#"<Button variant="primary" />"#).unwrap();

        assert_eq!(jsx.component, "Button");
        assert!(jsx.self_closing);
        assert_eq!(
            jsx.props.get("variant"),
            Some(&PropValue::String("primary".to_string()))
        );
        assert!(jsx.children.is_none());
    }

    #[test]
    fn parses_with_children() {
        let jsx = parse_inline_jsx(r#"<Button variant="default">Click me</Button>"#).unwrap();

        assert_eq!(jsx.component, "Button");
        assert!(!jsx.self_closing);
        assert_eq!(
            jsx.props.get("variant"),
            Some(&PropValue::String("default".to_string()))
        );
        assert_eq!(jsx.children, Some("Click me".to_string()));
    }

    #[test]
    fn parses_boolean_props() {
        let jsx = parse_inline_jsx(r#"<Button disabled>Disabled</Button>"#).unwrap();

        assert_eq!(jsx.props.get("disabled"), Some(&PropValue::Boolean(true)));
    }

    #[test]
    fn parses_expression_props() {
        let jsx = parse_inline_jsx(r#"<Button data={someValue}>Click</Button>"#).unwrap();

        assert_eq!(jsx.component, "Button");
        assert_eq!(jsx.children, Some("Click".to_string()));
        assert!(matches!(
            jsx.props.get("data"),
            Some(PropValue::Expression(_))
        ));
    }

    #[test]
    fn converts_to_custom_element() {
        let jsx = parse_inline_jsx(r#"<Button variant="primary" disabled>Click</Button>"#).unwrap();
        let html = to_custom_element(&jsx, "button-preview");

        assert!(html.contains("button-preview"));
        assert!(html.contains(r#"variant="primary""#));
        assert!(html.contains("disabled"));
        assert!(html.contains("Click"));
    }

    #[test]
    fn handles_empty_element() {
        let jsx = parse_inline_jsx(r#"<Icon name="star" />"#).unwrap();

        assert_eq!(jsx.component, "Icon");
        assert!(jsx.self_closing);
        assert_eq!(
            jsx.props.get("name"),
            Some(&PropValue::String("star".to_string()))
        );
    }

    #[test]
    fn parses_arrow_function_in_props() {
        let jsx =
            parse_inline_jsx(r#"<Button onClick={() => alert('hi')}>Click</Button>"#).unwrap();

        assert_eq!(jsx.component, "Button");
        assert_eq!(jsx.children, Some("Click".to_string()));
        match jsx.props.get("onClick") {
            Some(PropValue::Expression(expr)) => {
                assert!(
                    expr.contains("=>"),
                    "Expected arrow function in expression, got: {}",
                    expr
                );
                assert!(expr.contains("alert"));
            }
            other => panic!("Expected Expression prop, got: {:?}", other),
        }
    }

    #[test]
    fn parses_multi_element_with_comments() {
        // This is the pattern from rafters component docs (e.g., button.md)
        let source = r#"// Primary action
<Button variant="default">Save Changes</Button>

// Destructive action
<Button variant="destructive">Delete Account</Button>

// Loading state
<Button loading>Processing...</Button>"#;

        let results = parse_inline_jsx_all(source);
        assert_eq!(
            results.len(),
            3,
            "Expected 3 JSX elements, got {}",
            results.len()
        );
        assert_eq!(results[0].component, "Button");
        assert_eq!(
            results[0].props.get("variant"),
            Some(&PropValue::String("default".to_string()))
        );
        assert_eq!(
            results[1].props.get("variant"),
            Some(&PropValue::String("destructive".to_string()))
        );
        assert!(results[2].props.contains_key("loading"));
    }

    #[test]
    fn parses_multi_element_no_comments() {
        let source = r#"<Button variant="primary">Primary</Button>
<Button variant="secondary">Secondary</Button>"#;

        let results = parse_inline_jsx_all(source);
        assert_eq!(
            results.len(),
            2,
            "Expected 2 JSX elements, got {}",
            results.len()
        );
    }

    #[test]
    fn parses_member_expression_component() {
        // ContextMenu.Trigger pattern from rafters docs
        let source = r#"<ContextMenu>
  <ContextMenu.Trigger>
    <div>Right-click me</div>
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    <ContextMenu.Item>Edit</ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu>"#;

        let results = parse_inline_jsx_all(source);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].component, "ContextMenu");
    }

    #[test]
    fn parses_nested_same_name_components() {
        let jsx = parse_inline_jsx(r#"<Card><Card>inner</Card></Card>"#).unwrap();

        assert_eq!(jsx.component, "Card");
        assert!(!jsx.self_closing);
        // The children should contain the nested <Card>inner</Card>
        let children = jsx.children.as_deref().unwrap();
        assert!(
            children.contains("inner"),
            "Expected nested content, got: {}",
            children
        );
        assert!(
            children.contains("Card"),
            "Expected nested Card tag, got: {}",
            children
        );
    }
}
