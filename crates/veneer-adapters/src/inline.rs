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
use oxc_span::SourceType;

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
    let source = source.trim();

    let allocator = Allocator::default();
    let source_type = SourceType::jsx();
    let ret = oxc_parser::Parser::new(&allocator, source, source_type).parse();

    // If the parser panicked, we cannot extract anything useful.
    if ret.panicked {
        return None;
    }

    // Look for the first expression statement containing a JSX element.
    for stmt in &ret.program.body {
        if let Statement::ExpressionStatement(expr_stmt) = stmt {
            match &expr_stmt.expression {
                Expression::JSXElement(el) => {
                    return Some(extract_jsx_element(el, source));
                }
                _ => continue,
            }
        }
    }

    None
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
                            // Get the source text of the expression (inside the braces).
                            let span = get_jsx_expression_span(expr);
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

/// Get the span of a JSXExpression (which inherits Expression variants).
fn get_jsx_expression_span(expr: &JSXExpression<'_>) -> oxc_span::Span {
    use oxc_span::GetSpan;
    expr.span()
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
    let first_span = child_span(&children[0]);
    let last_span = child_span(&children[children.len() - 1]);

    let start = first_span.start as usize;
    let end = last_span.end as usize;
    let text = source[start..end].trim();

    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// Get the span of a JSXChild.
fn child_span(child: &JSXChild<'_>) -> oxc_span::Span {
    use oxc_span::GetSpan;
    child.span()
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
