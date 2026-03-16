//! Asset pipeline for CSS and JavaScript processing.

/// Asset pipeline utilities.
pub struct AssetPipeline;

impl AssetPipeline {
    /// Generate the main CSS file.
    pub fn generate_css() -> String {
        include_str!("../assets/main.css").to_string()
    }

    /// Generate the main JavaScript file.
    pub fn generate_js() -> String {
        include_str!("../assets/main.js").to_string()
    }

    /// Minify CSS using lightningcss.
    pub fn minify_css(css: &str) -> Result<String, String> {
        use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

        let stylesheet = StyleSheet::parse(css, ParserOptions::default())
            .map_err(|e| format!("CSS parse error: {}", e))?;

        let minified = stylesheet
            .to_css(PrinterOptions {
                minify: true,
                ..Default::default()
            })
            .map_err(|e| format!("CSS minify error: {}", e))?;

        Ok(minified.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_css() {
        let css = AssetPipeline::generate_css();
        assert!(css.contains(":root"));
        assert!(css.contains("--veneer-background"));
        assert!(css.contains("--veneer-primary"));
    }

    #[test]
    fn generates_js() {
        let js = AssetPipeline::generate_js();
        assert!(js.contains("addEventListener"));
        assert!(js.contains("clipboard"));
    }

    #[test]
    fn minifies_css() {
        let css = r#"
.button {
    background-color: blue;
    padding: 10px;
}
        "#;

        let minified = AssetPipeline::minify_css(css).unwrap();

        assert!(!minified.contains('\n'));
        assert!(minified.contains(".button"));
    }

    #[test]
    fn css_has_no_unnamespaced_vars() {
        let css = AssetPipeline::generate_css();
        for line in css.lines() {
            if let Some(pos) = line.find("var(--") {
                let after_var = &line[pos + 4..];
                assert!(
                    after_var.starts_with("--veneer-"),
                    "Found unnamespaced CSS variable in line: {}",
                    line.trim()
                );
            }
        }
    }

    #[test]
    fn css_has_concrete_color_values() {
        let css = AssetPipeline::generate_css();
        assert!(css.contains("--veneer-background: #ffffff"));
        assert!(css.contains("--veneer-foreground: #09090b"));
        assert!(css.contains("--veneer-primary: #2563eb"));
        assert!(css.contains("--veneer-border: #e4e4e7"));
    }
}
