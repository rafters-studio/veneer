//! React/JSX adapter for transforming components to Web Components.
//!
//! Uses oxc AST parsing instead of regex for reliable component extraction.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    BinaryOperator, BindingPatternKind, Declaration, Expression, ObjectPropertyKind, PropertyKey,
    Statement, TSSignature,
};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::generator::generate_web_component;
use crate::traits::{FrameworkAdapter, TransformContext, TransformError, TransformedBlock};

/// Extracted component structure from source code.
#[derive(Debug, Clone, Default)]
pub struct ComponentStructure {
    /// Component name (e.g., "Button")
    pub name: String,

    /// Variant classes mapping
    pub variant_lookup: Vec<(String, String)>,

    /// Size classes mapping
    pub size_lookup: Vec<(String, String)>,

    /// Base classes applied to all variants
    pub base_classes: String,

    /// Classes applied when disabled
    pub disabled_classes: String,

    /// Default variant value
    pub default_variant: String,

    /// Default size value
    pub default_size: String,

    /// Observed attributes from props
    pub observed_attributes: Vec<String>,
}

/// React/JSX to Web Component adapter.
#[derive(Debug, Default)]
pub struct ReactAdapter;

impl ReactAdapter {
    /// Create a new React adapter.
    pub fn new() -> Self {
        Self
    }

    /// Extract component structure from source code using oxc AST parsing.
    pub fn extract_structure(&self, source: &str) -> Result<ComponentStructure, TransformError> {
        let allocator = Allocator::default();
        let source_type = SourceType::tsx();
        let ret = Parser::new(&allocator, source, source_type).parse();

        if ret.panicked {
            return Err(TransformError::ParseError(
                "oxc parser panicked while parsing source".to_string(),
            ));
        }

        let program = &ret.program;

        let mut variant_lookup: Vec<(String, String)> = Vec::new();
        let mut size_lookup: Vec<(String, String)> = Vec::new();
        let mut base_classes: Option<String> = None;
        let mut disabled_classes: Option<String> = None;
        let mut component_name: Option<String> = None;
        let mut observed_attributes: Vec<String> = Vec::new();

        // Walk top-level statements
        for stmt in &program.body {
            Self::visit_statement(
                stmt,
                &mut variant_lookup,
                &mut size_lookup,
                &mut base_classes,
                &mut disabled_classes,
                &mut component_name,
                &mut observed_attributes,
            );
        }

        if variant_lookup.is_empty() {
            return Err(TransformError::MissingVariants);
        }

        let default_variant = variant_lookup
            .first()
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "default".to_string());

        let default_size = size_lookup
            .first()
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "default".to_string());

        Ok(ComponentStructure {
            name: component_name.unwrap_or_else(|| "Component".to_string()),
            base_classes: base_classes.unwrap_or_default(),
            disabled_classes: disabled_classes
                .unwrap_or_else(|| "opacity-50 pointer-events-none cursor-not-allowed".to_string()),
            variant_lookup,
            size_lookup,
            default_variant,
            default_size,
            observed_attributes,
        })
    }

    /// Process a single statement, extracting relevant declarations.
    fn visit_statement(
        stmt: &Statement<'_>,
        variant_lookup: &mut Vec<(String, String)>,
        size_lookup: &mut Vec<(String, String)>,
        base_classes: &mut Option<String>,
        disabled_classes: &mut Option<String>,
        component_name: &mut Option<String>,
        observed_attributes: &mut Vec<String>,
    ) {
        match stmt {
            // Handle: const variantClasses = { ... }
            // Handle: const baseClasses = '...'
            Statement::VariableDeclaration(decl) => {
                Self::visit_variable_declaration(
                    decl,
                    variant_lookup,
                    size_lookup,
                    base_classes,
                    disabled_classes,
                    component_name,
                );
            }

            // Handle: export function Button() {}
            Statement::FunctionDeclaration(func) => {
                if let Some(ref id) = func.id {
                    let name = id.name.as_str();
                    if is_pascal_case(name) && component_name.is_none() {
                        *component_name = Some(name.to_string());
                    }
                    // Extract props from function parameters
                    extract_params_attributes(&func.params, observed_attributes);
                }
            }

            // Handle: interface ButtonProps { ... }
            Statement::TSInterfaceDeclaration(iface) => {
                let iface_name = iface.id.name.as_str();
                if iface_name.ends_with("Props") {
                    extract_interface_attributes(iface, observed_attributes);
                }
            }

            // Handle: export const/function/default ...
            Statement::ExportNamedDeclaration(export) => {
                if let Some(ref decl) = export.declaration {
                    // Wrap declaration as a statement for recursive processing
                    match decl {
                        Declaration::VariableDeclaration(var_decl) => {
                            Self::visit_variable_declaration(
                                var_decl,
                                variant_lookup,
                                size_lookup,
                                base_classes,
                                disabled_classes,
                                component_name,
                            );
                        }
                        Declaration::FunctionDeclaration(func) => {
                            if let Some(ref id) = func.id {
                                let name = id.name.as_str();
                                if is_pascal_case(name) && component_name.is_none() {
                                    *component_name = Some(name.to_string());
                                }
                                extract_params_attributes(&func.params, observed_attributes);
                            }
                        }
                        Declaration::TSInterfaceDeclaration(iface) => {
                            let iface_name = iface.id.name.as_str();
                            if iface_name.ends_with("Props") {
                                extract_interface_attributes(iface, observed_attributes);
                            }
                        }
                        _ => {}
                    }
                }
            }

            Statement::ExportDefaultDeclaration(export) => {
                use oxc_ast::ast::ExportDefaultDeclarationKind;
                if let ExportDefaultDeclarationKind::FunctionDeclaration(func) = &export.declaration
                {
                    if let Some(ref id) = func.id {
                        let name = id.name.as_str();
                        if is_pascal_case(name) && component_name.is_none() {
                            *component_name = Some(name.to_string());
                        }
                        extract_params_attributes(&func.params, observed_attributes);
                    }
                }
            }

            _ => {}
        }
    }

    /// Process a variable declaration, looking for known identifiers.
    fn visit_variable_declaration(
        decl: &oxc_ast::ast::VariableDeclaration<'_>,
        variant_lookup: &mut Vec<(String, String)>,
        size_lookup: &mut Vec<(String, String)>,
        base_classes: &mut Option<String>,
        disabled_classes: &mut Option<String>,
        component_name: &mut Option<String>,
    ) {
        for declarator in &decl.declarations {
            let name = match &declarator.id.kind {
                BindingPatternKind::BindingIdentifier(id) => id.name.as_str(),
                _ => continue,
            };

            let Some(ref init) = declarator.init else {
                continue;
            };

            // Unwrap `as const`, `satisfies Type`, and `as Type` expressions
            let init = unwrap_type_expressions(init);

            match name {
                "variantClasses" => {
                    if let Some(entries) = extract_object_entries(init) {
                        *variant_lookup = entries;
                    }
                }
                "sizeClasses" => {
                    if let Some(entries) = extract_object_entries(init) {
                        *size_lookup = entries;
                    }
                }
                "baseClasses" => {
                    if let Some(value) = extract_string_value(init) {
                        *base_classes = Some(normalize_whitespace(&value));
                    }
                }
                "disabledClasses" | "disabledCls" => {
                    if let Some(value) = extract_string_value(init) {
                        *disabled_classes = Some(value);
                    }
                }
                _ => {
                    // Check for PascalCase component name from arrow function / function expression
                    if is_pascal_case(name) && component_name.is_none() {
                        match init {
                            Expression::ArrowFunctionExpression(_)
                            | Expression::FunctionExpression(_) => {
                                *component_name = Some(name.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

impl FrameworkAdapter for ReactAdapter {
    fn name(&self) -> &'static str {
        "react"
    }

    fn extensions(&self) -> &[&'static str] {
        &["tsx", "jsx"]
    }

    fn transform(
        &self,
        source: &str,
        tag_name: &str,
        _ctx: &TransformContext,
    ) -> Result<TransformedBlock, TransformError> {
        let structure = self.extract_structure(source)?;

        // Collect all classes used
        let mut classes_used: Vec<String> = Vec::new();

        // Add base classes
        for class in structure.base_classes.split_whitespace() {
            if !classes_used.contains(&class.to_string()) {
                classes_used.push(class.to_string());
            }
        }

        // Add variant classes
        for (_, classes) in &structure.variant_lookup {
            for class in classes.split_whitespace() {
                if !classes_used.contains(&class.to_string()) {
                    classes_used.push(class.to_string());
                }
            }
        }

        // Add size classes
        for (_, classes) in &structure.size_lookup {
            for class in classes.split_whitespace() {
                if !classes_used.contains(&class.to_string()) {
                    classes_used.push(class.to_string());
                }
            }
        }

        // Add disabled classes
        for class in structure.disabled_classes.split_whitespace() {
            if !classes_used.contains(&class.to_string()) {
                classes_used.push(class.to_string());
            }
        }

        // Generate the Web Component
        let web_component = generate_web_component(tag_name, &structure);

        Ok(TransformedBlock {
            web_component,
            tag_name: tag_name.to_string(),
            classes_used,
            attributes: structure.observed_attributes,
        })
    }
}

/// Unwrap TSAsExpression, TSSatisfiesExpression, and TSTypeAssertion to get the inner expression.
fn unwrap_type_expressions<'a>(expr: &'a Expression<'a>) -> &'a Expression<'a> {
    match expr {
        Expression::TSAsExpression(as_expr) => unwrap_type_expressions(&as_expr.expression),
        Expression::TSSatisfiesExpression(sat_expr) => {
            unwrap_type_expressions(&sat_expr.expression)
        }
        Expression::TSTypeAssertion(ta_expr) => unwrap_type_expressions(&ta_expr.expression),
        Expression::ParenthesizedExpression(paren) => unwrap_type_expressions(&paren.expression),
        other => other,
    }
}

/// Extract key-value pairs from an ObjectExpression.
fn extract_object_entries(expr: &Expression<'_>) -> Option<Vec<(String, String)>> {
    let obj = match expr {
        Expression::ObjectExpression(obj) => obj,
        _ => return None,
    };

    let mut entries = Vec::new();

    for prop in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };

        let key = match &prop.key {
            PropertyKey::StaticIdentifier(id) => id.name.as_str().to_string(),
            PropertyKey::StringLiteral(s) => s.value.as_str().to_string(),
            _ => continue,
        };

        let value_expr = unwrap_type_expressions(&prop.value);

        if let Some(value) = extract_string_value(value_expr) {
            entries.push((key, value));
        }
    }

    Some(entries)
}

/// Extract a string value from an expression.
/// Handles StringLiteral, BinaryExpression (concatenation), and TemplateLiteral (no interpolation).
fn extract_string_value(expr: &Expression<'_>) -> Option<String> {
    match expr {
        Expression::StringLiteral(s) => Some(s.value.as_str().to_string()),

        Expression::TemplateLiteral(tpl) => {
            // Only handle template literals with no interpolated expressions
            if tpl.expressions.is_empty() && !tpl.quasis.is_empty() {
                let value = tpl
                    .quasis
                    .iter()
                    .map(|q| q.value.raw.as_str())
                    .collect::<Vec<_>>()
                    .join("");
                Some(value)
            } else {
                None
            }
        }

        Expression::BinaryExpression(bin) => {
            if bin.operator == BinaryOperator::Addition {
                let left = extract_string_value(&bin.left)?;
                let right = extract_string_value(&bin.right)?;
                Some(format!("{left}{right}"))
            } else {
                None
            }
        }

        // Unwrap type expressions in case they were not already unwrapped
        Expression::TSAsExpression(as_expr) => extract_string_value(&as_expr.expression),
        Expression::TSSatisfiesExpression(sat) => extract_string_value(&sat.expression),
        Expression::ParenthesizedExpression(paren) => extract_string_value(&paren.expression),

        _ => None,
    }
}

/// Check if a string is PascalCase (starts with uppercase letter).
fn is_pascal_case(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Normalize whitespace in a string (collapse multiple spaces, trim).
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract observed attributes from a TSInterfaceDeclaration whose name ends with "Props".
fn extract_interface_attributes(
    iface: &oxc_ast::ast::TSInterfaceDeclaration<'_>,
    attrs: &mut Vec<String>,
) {
    for sig in &iface.body.body {
        if let TSSignature::TSPropertySignature(prop) = sig {
            let name = match &prop.key {
                PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                _ => continue,
            };

            if should_include_attribute(name) && !attrs.contains(&name.to_string()) {
                attrs.push(name.to_string());
            }
        }
    }
}

/// Extract observed attributes from function parameters (destructured object pattern).
fn extract_params_attributes(params: &oxc_ast::ast::FormalParameters<'_>, attrs: &mut Vec<String>) {
    for param in &params.items {
        if let BindingPatternKind::ObjectPattern(obj_pat) = &param.pattern.kind {
            for prop in &obj_pat.properties {
                let name = match &prop.key {
                    PropertyKey::StaticIdentifier(id) => id.name.as_str(),
                    _ => continue,
                };

                if should_include_attribute(name) && !attrs.contains(&name.to_string()) {
                    attrs.push(name.to_string());
                }
            }
        }
    }
}

/// Determine if an attribute name should be included in observed attributes.
/// Excludes React-specific props that have no Web Component equivalent.
fn should_include_attribute(name: &str) -> bool {
    !name.is_empty() && name != "children" && name != "className" && name != "style"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_variant_classes() {
        let source = r#"
const variantClasses: Record<string, string> = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
};

export function Button() {
  return <button />;
}
        "#;

        let adapter = ReactAdapter::new();
        let result = adapter
            .transform(source, "button-preview", &TransformContext::default())
            .unwrap();

        assert!(result.web_component.contains("variantClasses"));
        assert!(result.web_component.contains("bg-primary"));
        assert!(result.classes_used.contains(&"bg-primary".to_string()));
    }

    #[test]
    fn extracts_concatenated_base_classes() {
        let source = r#"
const variantClasses = { default: '' };
const baseClasses =
  'inline-flex items-center ' +
  'justify-center gap-2';

export function Button() {}
        "#;

        let adapter = ReactAdapter::new();
        let structure = adapter.extract_structure(source).unwrap();

        assert!(structure.base_classes.contains("inline-flex"));
        assert!(structure.base_classes.contains("items-center"));
        assert!(structure.base_classes.contains("justify-center"));
    }

    #[test]
    fn extracts_simple_base_classes() {
        let source = r#"
const variantClasses = { default: '' };
const baseClasses = 'inline-flex items-center';

export function Button() {}
        "#;

        let adapter = ReactAdapter::new();
        let structure = adapter.extract_structure(source).unwrap();

        assert_eq!(structure.base_classes, "inline-flex items-center");
    }

    #[test]
    fn errors_on_missing_variants() {
        let source = "export function Button() { return <button />; }";

        let adapter = ReactAdapter::new();
        let result = adapter.transform(source, "button-preview", &TransformContext::default());

        assert!(matches!(result, Err(TransformError::MissingVariants)));
    }

    #[test]
    fn extracts_observed_attributes() {
        let source = r#"
const variantClasses = { default: '' };

interface ButtonProps {
  variant?: string;
  size?: string;
  disabled?: boolean;
  loading?: boolean;
}

export function Button({ variant, size, disabled, loading }: ButtonProps) {}
        "#;

        let adapter = ReactAdapter::new();
        let result = adapter
            .transform(source, "button-preview", &TransformContext::default())
            .unwrap();

        assert!(result.attributes.contains(&"variant".to_string()));
        assert!(result.attributes.contains(&"size".to_string()));
        assert!(result.attributes.contains(&"disabled".to_string()));
        assert!(result.attributes.contains(&"loading".to_string()));
    }

    #[test]
    fn generates_valid_tag_name() {
        let source = r#"
const variantClasses = { primary: 'bg-blue-500' };
export function Button() {}
        "#;

        let adapter = ReactAdapter::new();
        let result = adapter
            .transform(source, "my-button", &TransformContext::default())
            .unwrap();

        assert_eq!(result.tag_name, "my-button");
        assert!(result.web_component.contains("my-button"));
    }

    #[test]
    fn handles_as_const_satisfies_pattern() {
        let source = r#"
const variantClasses = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
} as const satisfies Record<string, string>;

export function Button() {}
        "#;

        let adapter = ReactAdapter::new();
        let structure = adapter.extract_structure(source).unwrap();

        assert_eq!(structure.variant_lookup.len(), 2);
        assert_eq!(
            structure.variant_lookup[0],
            (
                "default".to_string(),
                "bg-primary text-primary-foreground".to_string()
            )
        );
        assert_eq!(
            structure.variant_lookup[1],
            (
                "secondary".to_string(),
                "bg-secondary text-secondary-foreground".to_string()
            )
        );
    }

    #[test]
    fn handles_comments_inside_objects() {
        let source = r#"
const variantClasses = {
  // Primary variant for main actions
  default: 'bg-primary text-primary-foreground',
  /* Secondary variant for less important actions */
  secondary: 'bg-secondary text-secondary-foreground',
};

export function Button() {}
        "#;

        let adapter = ReactAdapter::new();
        let structure = adapter.extract_structure(source).unwrap();

        assert_eq!(structure.variant_lookup.len(), 2);
        assert_eq!(
            structure.variant_lookup[0],
            (
                "default".to_string(),
                "bg-primary text-primary-foreground".to_string()
            )
        );
    }

    #[test]
    fn handles_template_literals_in_class_values() {
        let source = r#"
const variantClasses = {
  default: `bg-primary text-primary-foreground`,
  secondary: `bg-secondary text-secondary-foreground`,
};

const baseClasses = `inline-flex items-center`;

export function Button() {}
        "#;

        let adapter = ReactAdapter::new();
        let structure = adapter.extract_structure(source).unwrap();

        assert_eq!(structure.variant_lookup.len(), 2);
        assert_eq!(
            structure.variant_lookup[0],
            (
                "default".to_string(),
                "bg-primary text-primary-foreground".to_string()
            )
        );
        assert_eq!(structure.base_classes, "inline-flex items-center");
    }
}
