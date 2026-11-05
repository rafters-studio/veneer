//! T18: the rafters check's own rules on oxc. A toy; see README.md.
//!
//! t18-oxc-check [root] [--stdin <path>] [--jsonl]
//! The report on stdout, notices on stderr. Exit 0 for no error, 1 for an error, 2 when it could not run.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use oxc_allocator::{Allocator, CloneIn as _};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk, Visit};
use oxc_parser::Parser;
use oxc_resolver::{
    ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions, TsconfigReferences,
};
use oxc_semantic::SymbolId;
use oxc_semantic::{Semantic, SemanticBuilder, SemanticBuilderAstroExt};
use oxc_span::{GetSpan, SourceType, Span};

const INTELLIGENCE_TAGS: [&str; 6] = [
    "cognitive-load",
    "attention-economics",
    "trust-building",
    "accessibility",
    "semantic-meaning",
    "usage-patterns",
];
const MAX_RESOLVE_STEPS: usize = 8;
const FREEHAND_HELP: &str = "use Container or Grid for layout";
const AUTHORED_FILL: &str = "a fill the designer authors, fill=\"bg-fill-<name>\"";

// ---- One parsed file. Leaked: the process is one check, and every file lives to its end.

struct Parsed {
    path: PathBuf,
    source: &'static str,
    astro: Option<&'static AstroRoot<'static>>,
    program: Option<&'static Program<'static>>,
    semantic: Semantic<'static>,
    line_starts: Vec<usize>,
}

impl Parsed {
    fn is_astro(&self) -> bool {
        self.astro.is_some()
    }

    fn body(&self) -> Option<&'static Program<'static>> {
        self.program.or_else(|| {
            self.astro
                .and_then(|a| a.frontmatter.as_ref().map(|f| &f.program))
        })
    }

    // 1-based line, and 1-based column in UTF-16 code units, as the language server counts.
    fn position(&self, offset: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&s| s <= offset) - 1;
        let start = self.line_starts[line];
        let column = self.source[start..offset].encode_utf16().count();
        (line + 1, column + 1)
    }

    fn utf16_width(&self, span: Span) -> usize {
        self.source[span.start as usize..span.end as usize]
            .encode_utf16()
            .count()
    }

    fn symbol_of(&self, ident: &IdentifierReference) -> Option<SymbolId> {
        let reference = ident.reference_id.get()?;
        self.semantic.scoping().get_reference(reference).symbol_id()
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut starts = vec![0];
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'\r' if bytes.get(at + 1) == Some(&b'\n') => {
                at += 2;
                starts.push(at);
            }
            b'\r' | b'\n' => {
                at += 1;
                starts.push(at);
            }
            _ => at += 1,
        }
    }
    starts
}

fn parse(path: &Path, text: String) -> Option<Parsed> {
    let allocator: &'static Allocator = Box::leak(Box::default());
    let source: &'static str = Box::leak(text.into_boxed_str());
    let name = path.to_string_lossy();
    if name.ends_with(".astro") {
        let ret = Parser::new(allocator, source, SourceType::astro()).parse_astro();
        if ret.panicked {
            return None;
        }
        let root: &'static AstroRoot<'static> = allocator.alloc(ret.root);
        let comments: &'static oxc_allocator::Vec<'static, Comment> =
            allocator.alloc(root.frontmatter.as_ref().map_or_else(
                || oxc_allocator::Vec::new_in(allocator),
                |f| f.program.comments.clone_in(allocator),
            ));
        let semantic = SemanticBuilder::new()
            .build_astro(root, source, comments)
            .semantic;
        return Some(Parsed {
            path: path.to_path_buf(),
            source,
            astro: Some(root),
            program: None,
            semantic,
            line_starts: line_starts(source),
        });
    }
    let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::tsx());
    let ret = Parser::new(allocator, source, source_type).parse();
    if ret.panicked {
        return None;
    }
    let program: &'static Program<'static> = allocator.alloc(ret.program);
    let semantic = SemanticBuilder::new().build(program).semantic;
    Some(Parsed {
        path: path.to_path_buf(),
        source,
        astro: None,
        program: Some(program),
        semantic,
        line_starts: line_starts(source),
    })
}


// ---- The project.

#[derive(Clone, Copy, PartialEq)]
enum Scan {
    Format,
    System,
    All,
}

struct Project {
    root: PathBuf,
    folders: Vec<PathBuf>,
    scan: Scan,
    resolver: Resolver,
    unsaved: Option<(PathBuf, String)>,
    cache: RefCell<HashMap<PathBuf, Option<&'static Parsed>>>,
    surfaces: RefCell<Option<Result<HashSet<String>, String>>>,
}

fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl Project {
    fn open(root: &Path, unsaved: Option<(PathBuf, String)>) -> Result<Project, String> {
        let root = real(root);
        let config_path = root.join(".rafters/config.rafters.json");
        let text = std::fs::read_to_string(&config_path)
            .map_err(|_| ".rafters/config.rafters.json not found".to_string())?;
        let config: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| format!(".rafters/config.rafters.json: {e}"))?;
        let entries: Vec<String> = match &config["componentsPath"] {
            serde_json::Value::String(s) => vec![s.clone()],
            serde_json::Value::Array(list) => list
                .iter()
                .filter_map(|e| match e {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Object(o) if o.get("root").is_none_or(|r| r == true) => {
                        o.get("path").and_then(|p| p.as_str()).map(str::to_string)
                    }
                    _ => None,
                })
                .collect(),
            _ => return Err("componentsPath: the field is missing".to_string()),
        };
        let folders = entries.iter().map(|e| real(&root.join(e))).collect();
        let scan = match config["veneer"]["scan"].as_str() {
            Some("system") => Scan::System,
            Some("all") => Scan::All,
            _ => Scan::Format,
        };
        let tsconfig = root.join("tsconfig.json");
        let resolver = Resolver::new(ResolveOptions {
            tsconfig: tsconfig.exists().then(|| {
                TsconfigDiscovery::Manual(TsconfigOptions {
                    config_file: tsconfig,
                    references: TsconfigReferences::Auto,
                })
            }),
            extensions: [".ts", ".tsx", ".d.ts", ".js", ".jsx", ".mjs", ".astro"]
                .map(String::from)
                .to_vec(),
            extension_alias: vec![(
                ".js".into(),
                vec![".ts".into(), ".tsx".into(), ".js".into()],
            )],
            condition_names: ["types", "import", "default"].map(String::from).to_vec(),
            main_fields: ["types", "module", "main"].map(String::from).to_vec(),
            ..ResolveOptions::default()
        });
        Ok(Project {
            root,
            folders,
            scan,
            resolver,
            unsaved,
            cache: RefCell::default(),
            surfaces: RefCell::default(),
        })
    }

    fn file(&self, path: &Path) -> Option<&'static Parsed> {
        let path = real(path);
        if let Some(found) = self.cache.borrow().get(&path) {
            return *found;
        }
        let text = match &self.unsaved {
            Some((p, t)) if *p == path => Some(t.clone()),
            _ => std::fs::read_to_string(&path).ok(),
        };
        let parsed = text
            .and_then(|t| parse(&path, t))
            .map(|p| &*Box::leak(Box::new(p)));
        self.cache.borrow_mut().insert(path, parsed);
        parsed
    }

    fn resolve_module(&self, from: &Parsed, specifier: &str) -> Option<&'static Parsed> {
        let dir = from.path.parent()?;
        let resolved = self.resolver.resolve(dir, specifier).ok()?;
        self.file(resolved.path())
    }

    fn is_under_folder(&self, path: &Path) -> bool {
        let path = real(path);
        self.folders.iter().any(|f| path.starts_with(f))
    }

    fn is_own_source(&self, path: &Path) -> bool {
        match real(path).strip_prefix(&self.root) {
            Ok(rest) => !rest.components().any(|c| c.as_os_str() == "node_modules"),
            Err(_) => false,
        }
    }

    // The rafters shape, read from the file's text and the folder's names; nothing here runs.
    fn qualifies(&self, file: &Parsed) -> bool {
        let name = file.path.to_string_lossy();
        let Some(stem) = name
            .strip_suffix(".tsx")
            .or_else(|| name.strip_suffix(".astro"))
        else {
            return false;
        };
        if !Path::new(&format!("{stem}.behavior.ts")).exists()
            || !Path::new(&format!("{stem}.classes.ts")).exists()
        {
            return false;
        }
        let Some(program) = file.body() else {
            return false;
        };
        program.comments.iter().any(|comment| {
            let text = &file.source[comment.span.start as usize..comment.span.end as usize];
            text.starts_with("/**") && INTELLIGENCE_TAGS.iter().all(|tag| holds_tag(text, tag))
        })
    }

    fn is_exempt(&self, file: &Parsed) -> bool {
        self.is_under_folder(&file.path) || self.qualifies(file)
    }

    fn is_checked_use(&self, declared_in: &Parsed) -> bool {
        match self.scan {
            Scan::System => self.is_under_folder(&declared_in.path),
            Scan::Format => self.is_exempt(declared_in),
            Scan::All => self.is_exempt(declared_in) || self.is_own_source(&declared_in.path),
        }
    }

    fn surfaces(&self) -> Result<HashSet<String>, String> {
        self.surfaces
            .borrow_mut()
            .get_or_insert_with(|| surface_words(&self.root.join(".rafters/tokens")))
            .clone()
    }
}

fn holds_tag(text: &str, tag: &str) -> bool {
    text.match_indices(&format!("@{tag}")).any(|(at, m)| {
        let next = text[at + m.len()..].chars().next();
        !next.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_')
    })
}

fn surface_words(dir: &Path) -> Result<HashSet<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut words = HashSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.to_string_lossy().ends_with(".rafters.json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        for token in value["tokens"].as_array().into_iter().flatten() {
            let name = token["name"].as_str().unwrap_or("");
            if token["namespace"] == "fill" {
                if let Some(word) = name.strip_prefix("surface-") {
                    words.insert(word.to_string());
                }
            }
        }
    }
    Ok(words)
}

// ---- Following a name to what declares it.

enum Found {
    // A const, let or var declarator, in the file that holds it.
    Declarator(&'static Parsed, &'static VariableDeclarator<'static>),
    // Declared by something else in this file: a function, a class, an .astro file's default.
    Elsewhere(&'static Parsed),
}

impl Found {
    fn file(&self) -> &'static Parsed {
        match self {
            Found::Declarator(f, _) | Found::Elsewhere(f) => f,
        }
    }
}

enum Exported {
    Symbol(&'static Parsed, SymbolId),
    File(&'static Parsed),
}

impl Project {
    fn follow(&self, mut file: &'static Parsed, mut symbol: SymbolId) -> Option<Found> {
        for _ in 0..MAX_RESOLVE_STEPS {
            let node = file.semantic.symbol_declaration(symbol);
            match node.kind() {
                AstKind::VariableDeclarator(declarator) => {
                    if let Some(Expression::Identifier(next)) = &declarator.init {
                        symbol = file.symbol_of(next)?;
                        continue;
                    }
                    return Some(Found::Declarator(file, declarator));
                }
                AstKind::ImportSpecifier(spec) => {
                    let AstKind::ImportDeclaration(decl) =
                        file.semantic.nodes().parent_kind(node.id())
                    else {
                        return None;
                    };
                    match self.exported(
                        file,
                        decl.source.value.as_str(),
                        spec.imported.name().as_str(),
                        0,
                    )? {
                        Exported::Symbol(f, s) => (file, symbol) = (f, s),
                        Exported::File(f) => return Some(Found::Elsewhere(f)),
                    }
                }
                AstKind::ImportDefaultSpecifier(_) => {
                    let AstKind::ImportDeclaration(decl) =
                        file.semantic.nodes().parent_kind(node.id())
                    else {
                        return None;
                    };
                    match self.exported(file, decl.source.value.as_str(), "default", 0)? {
                        Exported::Symbol(f, s) => (file, symbol) = (f, s),
                        Exported::File(f) => return Some(Found::Elsewhere(f)),
                    }
                }
                AstKind::ImportNamespaceSpecifier(_) => return None,
                _ => return Some(Found::Elsewhere(file)),
            }
        }
        None
    }

    fn namespace_target(&self, file: &'static Parsed, symbol: SymbolId) -> Option<&'static Parsed> {
        let node = file.semantic.symbol_declaration(symbol);
        let AstKind::ImportNamespaceSpecifier(_) = node.kind() else {
            return None;
        };
        let AstKind::ImportDeclaration(decl) = file.semantic.nodes().parent_kind(node.id()) else {
            return None;
        };
        self.resolve_module(file, decl.source.value.as_str())
    }

    // What `name` exported from the module `specifier` names, seen from `from`.
    fn exported(
        &self,
        from: &Parsed,
        specifier: &str,
        name: &str,
        depth: usize,
    ) -> Option<Exported> {
        let target = self.resolve_module(from, specifier)?;
        self.export_of(target, name, depth)
    }

    fn export_of(&self, target: &'static Parsed, name: &str, depth: usize) -> Option<Exported> {
        if depth > MAX_RESOLVE_STEPS {
            return None;
        }
        if target.is_astro() {
            return (name == "default").then_some(Exported::File(target));
        }
        let program = target.program?;
        for statement in &program.body {
            match statement {
                Statement::ExportNamedDeclaration(export) => {
                    if let Some(declaration) = &export.declaration {
                        if let Some(found) = declared_name(target, declaration, name) {
                            return Some(found);
                        }
                    }
                    for spec in &export.specifiers {
                        if spec.exported.name().as_str() != name {
                            continue;
                        }
                        if let Some(source) = &export.source {
                            return self.exported(
                                target,
                                source.value.as_str(),
                                spec.local.name().as_str(),
                                depth + 1,
                            );
                        }
                        if let ModuleExportName::IdentifierReference(local) = &spec.local {
                            return target.symbol_of(local).map(|s| Exported::Symbol(target, s));
                        }
                    }
                }
                Statement::ExportDefaultDeclaration(export) if name == "default" => {
                    return Some(match &export.declaration {
                        ExportDefaultDeclarationKind::Identifier(id) => {
                            match target.symbol_of(id) {
                                Some(s) => Exported::Symbol(target, s),
                                None => Exported::File(target),
                            }
                        }
                        ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                            match f.id.as_ref().and_then(|id| id.symbol_id.get()) {
                                Some(s) => Exported::Symbol(target, s),
                                None => Exported::File(target),
                            }
                        }
                        _ => Exported::File(target),
                    });
                }
                _ => {}
            }
        }
        for statement in &program.body {
            if let Statement::ExportAllDeclaration(all) = statement {
                if all.exported.is_none() {
                    if let Some(found) =
                        self.exported(target, all.source.value.as_str(), name, depth + 1)
                    {
                        return Some(found);
                    }
                }
            }
        }
        None
    }

    // The declaration a JSX tag names, for <Button>, <Tag> where const Tag = Button, and <UI.Button>.
    fn tag_file(&self, file: &'static Parsed, name: &JSXElementName) -> Option<&'static Parsed> {
        match name {
            JSXElementName::IdentifierReference(id) => {
                Some(self.follow(file, file.symbol_of(id)?)?.file())
            }
            JSXElementName::MemberExpression(member) => {
                let JSXMemberExpressionObject::IdentifierReference(object) = &member.object else {
                    return None;
                };
                let module = self.namespace_target(file, file.symbol_of(object)?)?;
                match self.export_of(module, member.property.name.as_str(), 0)? {
                    Exported::File(f) => Some(f),
                    Exported::Symbol(f, s) => Some(self.follow(f, s)?.file()),
                }
            }
            _ => None,
        }
    }
}

fn declared_name(
    target: &'static Parsed,
    declaration: &Declaration,
    name: &str,
) -> Option<Exported> {
    let symbol = match declaration {
        Declaration::VariableDeclaration(vars) => {
            vars.declarations.iter().find_map(|d| match &d.id {
                BindingPattern::BindingIdentifier(id) if id.name.as_str() == name => {
                    id.symbol_id.get()
                }
                _ => None,
            })
        }
        Declaration::FunctionDeclaration(f) => {
            f.id.as_ref()
                .filter(|id| id.name.as_str() == name)?
                .symbol_id
                .get()
        }
        Declaration::ClassDeclaration(c) => {
            c.id.as_ref()
                .filter(|id| id.name.as_str() == name)?
                .symbol_id
                .get()
        }
        _ => None,
    }?;
    Some(Exported::Symbol(target, symbol))
}

// ---- Rule two: a bracket value.

// The utility follows the last colon outside brackets, so a modifier's brackets are not its own.
fn utility_of(class_name: &str) -> &str {
    let mut depth = 0i32;
    let mut start = 0;
    for (at, c) in class_name.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ':' if depth == 0 => start = at + 1,
            _ => {}
        }
    }
    &class_name[start..]
}

// A string, or a template with no substitutions: TypeScript's StringLiteralLike.
fn string_like<'a>(e: &'a Expression<'a>) -> Option<(Span, String)> {
    match e {
        Expression::StringLiteral(s) => Some((s.span, s.value.to_string())),
        Expression::TemplateLiteral(t) if t.expressions.is_empty() => Some((
            t.span,
            t.quasis
                .first()
                .and_then(|q| q.value.cooked.as_ref())
                .map_or_else(String::new, |c| c.to_string()),
        )),
        _ => None,
    }
}

// A finding's anchor: the literal itself, or the use of a const written elsewhere.
#[derive(Clone, Copy)]
struct ClassString {
    at: Span,
    in_literal: bool,
    literal_file: &'static Parsed,
    literal: Span,
}

impl ClassString {
    fn key(&self) -> (Span, bool, usize, Span) {
        (
            self.at,
            self.in_literal,
            std::ptr::from_ref(self.literal_file) as usize,
            self.literal,
        )
    }
}

struct Collector<'p> {
    project: &'p Project,
    file: &'static Parsed,
    seen: HashSet<(Span, bool, usize, Span)>,
    found: Vec<ClassString>,
}

impl Collector<'_> {
    fn add(&mut self, s: ClassString) {
        if self.seen.insert(s.key()) {
            self.found.push(s);
        }
    }

    fn literal(&mut self, span: Span) {
        self.add(ClassString {
            at: span,
            in_literal: true,
            literal_file: self.file,
            literal: span,
        });
    }

    fn const_initializer(
        &self,
        ident: &IdentifierReference,
    ) -> Option<(&'static Parsed, &'static Expression<'static>)> {
        match self
            .project
            .follow(self.file, self.file.symbol_of(ident)?)?
        {
            Found::Declarator(f, d) if d.kind == VariableDeclarationKind::Const => {
                Some((f, d.init.as_ref()?))
            }
            _ => None,
        }
    }

    // What a use of a const in a class reaches: the literal itself in this file, or the use.
    fn reached(&mut self, at: Span, file: &'static Parsed, literal: Span, at_use: bool) {
        if !at_use && std::ptr::eq(file, self.file) {
            self.add(ClassString {
                at: literal,
                in_literal: true,
                literal_file: file,
                literal,
            });
        } else if std::ptr::eq(file, self.file) || self.project.is_own_source(&file.path) {
            self.add(ClassString {
                at,
                in_literal: false,
                literal_file: file,
                literal,
            });
        }
    }
}

impl<'a> Visit<'a> for Collector<'_> {
    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        self.literal(it.span);
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        if it.expressions.is_empty() {
            self.literal(it.span);
        } else {
            walk::walk_template_literal(self, it);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some((file, init)) = self.const_initializer(it) {
            if let Some((span, _)) = string_like(init) {
                self.reached(it.span, file, span, false);
            }
        }
    }

    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if let Expression::Identifier(object) = &it.object {
            if let Some((file, init)) = self.const_initializer(object) {
                let mut init = init;
                loop {
                    match init {
                        Expression::TSAsExpression(e) => init = &e.expression,
                        Expression::TSSatisfiesExpression(e) => init = &e.expression,
                        _ => break,
                    }
                }
                if let Expression::ObjectExpression(object) = init {
                    let property = object.properties.iter().find_map(|p| match p {
                        ObjectPropertyKind::ObjectProperty(p)
                            if p.kind == PropertyKind::Init
                                && !p.method
                                && !p.shorthand
                                && matches!(&p.key, PropertyKey::StaticIdentifier(k) if k.name.as_str() == it.property.name.as_str()) =>
                        {
                            Some(p)
                        }
                        _ => None,
                    });
                    if let Some(property) = property {
                        if let Some((span, _)) = string_like(&property.value) {
                            let before = self.found.len();
                            self.reached(it.span, file, span, true);
                            if self.found.len() > before
                                || std::ptr::eq(file, self.file)
                                || self.project.is_own_source(&file.path)
                            {
                                return;
                            }
                        }
                    }
                }
            }
        }
        walk::walk_static_member_expression(self, it);
    }
}

fn class_attribute(name: &JSXAttributeName) -> bool {
    match name {
        JSXAttributeName::Identifier(id) => {
            matches!(id.name.as_str(), "class" | "className" | "class:list")
        }
        JSXAttributeName::NamespacedName(n) => n.namespace.name == "class" && n.name.name == "list",
    }
}

// Every class string the file's class attributes and classy calls reach, or every literal in a classes file.
struct Finder<'c, 'p> {
    collector: &'c mut Collector<'p>,
    every_literal: bool,
}

impl<'a> Visit<'a> for Finder<'_, '_> {
    fn visit_astro_script(&mut self, _it: &AstroScript<'a>) {}

    fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
        if self.every_literal {
            self.collector.literal(it.span);
        }
    }

    fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
        if self.every_literal && it.expressions.is_empty() {
            self.collector.literal(it.span);
        }
        walk::walk_template_literal(self, it);
    }

    fn visit_jsx_attribute(&mut self, it: &JSXAttribute<'a>) {
        if !self.every_literal && class_attribute(&it.name) {
            if let Some(value) = &it.value {
                self.collector.visit_jsx_attribute_value(value);
            }
        }
        walk::walk_jsx_attribute(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !self.every_literal
            && matches!(&it.callee, Expression::Identifier(id) if id.name == "classy")
        {
            for argument in &it.arguments {
                self.collector.visit_argument(argument);
            }
        }
        walk::walk_call_expression(self, it);
    }
}

#[derive(Clone)]
struct Finding {
    file: String,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    severity: &'static str,
    code: &'static str,
    message: String,
    help: Option<String>,
}

fn walk_file<'a, V: Visit<'a>>(visitor: &mut V, file: &'a Parsed) {
    if let Some(root) = file.astro {
        visitor.visit_astro_root(root);
    } else if let Some(program) = file.program {
        visitor.visit_program(program);
    }
}

fn bracket_values(project: &Project, rel: &str, file: &'static Parsed) -> Vec<Finding> {
    let every_literal = file.path.to_string_lossy().ends_with(".classes.ts");
    if every_literal && !project.is_own_source(&file.path) {
        return vec![];
    }
    let mut collector = Collector {
        project,
        file,
        seen: HashSet::new(),
        found: vec![],
    };
    walk_file(
        &mut Finder {
            collector: &mut collector,
            every_literal,
        },
        file,
    );
    let mut out = vec![];
    for s in collector.found {
        let literal_file = s.literal_file;
        // The literal as written, so an offset in it is an offset in the file.
        let written =
            &literal_file.source[s.literal.start as usize + 1..s.literal.end as usize - 1];
        for (index, class_name) in split_whitespace_indices(written) {
            if !utility_of(class_name).contains('[') {
                continue;
            }
            let (offset, width) = if s.in_literal {
                (
                    s.at.start as usize + 1 + index,
                    class_name.encode_utf16().count(),
                )
            } else {
                (s.at.start as usize, file.utf16_width(s.at))
            };
            let (line, column) = file.position(offset);
            out.push(Finding {
                file: rel.to_string(),
                line,
                column,
                end_line: line,
                end_column: column + width,
                severity: "error",
                code: "rafters(arbitrary-value)",
                message: format!("{class_name} is an arbitrary value"),
                help: None,
            });
        }
    }
    out
}

// JavaScript's /\S+/g: runs of characters that are not whitespace or line terminators.
fn split_whitespace_indices(text: &str) -> Vec<(usize, &str)> {
    let is_space = |c: char| c.is_whitespace() || c == '\u{feff}';
    let mut out = vec![];
    let mut start = None;
    for (at, c) in text.char_indices() {
        match (is_space(c), start) {
            (true, Some(s)) => {
                out.push((s, &text[s..at]));
                start = None;
            }
            (false, None) => start = Some(at),
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, &text[s..]));
    }
    out
}

// ---- Rule three: a fill or surface value. Rule four: a freehand div.

fn is_div(name: &JSXElementName) -> bool {
    matches!(name, JSXElementName::Identifier(id) if id.name == "div")
}

fn fill_word(word: &str) -> bool {
    let mut parts = word.split('-');
    let first = parts.next().unwrap_or("");
    let lower_alnum = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    };
    first.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && lower_alnum(first)
        && parts.all(lower_alnum)
}

fn fill_stop(stop: &str) -> bool {
    let mut parts = stop.split('/');
    let word = parts.next().unwrap_or("");
    let alpha = parts.next();
    let alpha_ok = alpha.is_none_or(|a| {
        a == "100"
            || (!a.is_empty()
                && a.len() <= 2
                && a.chars().all(|c| c.is_ascii_digit())
                && !(a.len() == 2 && a.starts_with('0')))
    });
    fill_word(word) && alpha_ok && parts.next().is_none()
}

fn retired_fill_form(value: &str) -> Option<&'static str> {
    if value.starts_with("bg-fill-") {
        return None;
    }
    if fill_word(value) && !value.contains("-to-") {
        return Some("bare");
    }
    let stops: Vec<&str> = value.split("-to-").collect();
    (stops.len() <= 2 && stops.iter().all(|s| fill_stop(s))).then_some("composed")
}

fn fill_finding(project: &Project, value: &str) -> Option<(&'static str, String, Option<String>)> {
    if value.is_empty() || value.strip_prefix("bg-fill-").is_some_and(fill_word) {
        return None;
    }
    let Some(form) = retired_fill_form(value) else {
        return Some((
            "rafters(invalid-fill)",
            format!("fill=\"{value}\" is not exactly one bg-fill-* token"),
            None,
        ));
    };
    let help = if form == "composed" {
        AUTHORED_FILL.to_string()
    } else {
        match project.surfaces() {
            Err(e) => {
                format!("the project's tokens could not be loaded, so no replacement is named: {e}")
            }
            Ok(words) if words.contains(value) => format!("surface=\"{value}\""),
            Ok(_) => AUTHORED_FILL.to_string(),
        }
    };
    Some((
        "rafters(retired-fill)",
        format!("fill=\"{value}\" is a retired fill value"),
        Some(help),
    ))
}

fn surface_finding(
    project: &Project,
    value: &str,
) -> Option<(&'static str, String, Option<String>)> {
    if value.is_empty() {
        return None;
    }
    match project.surfaces() {
        Err(e) => Some((
            "rafters(unknown-surface)",
            format!("surface=\"{value}\" could not be checked"),
            Some(format!(
                "the project's tokens could not be loaded, so no surface is named: {e}"
            )),
        )),
        Ok(words) if words.contains(value) => None,
        Ok(_) => Some((
            "rafters(unknown-surface)",
            format!("surface=\"{value}\" is not a surface in the project's tokens"),
            None,
        )),
    }
}

fn value_literal(value: &JSXAttributeValue) -> Option<String> {
    match value {
        JSXAttributeValue::StringLiteral(s) => Some(s.value.to_string()),
        JSXAttributeValue::ExpressionContainer(c) => c
            .expression
            .as_expression()
            .and_then(string_like)
            .map(|(_, v)| v),
        _ => None,
    }
}

struct Elements<'p> {
    project: &'p Project,
    rel: &'p str,
    file: &'static Parsed,
    checked: HashMap<Span, bool>,
    divs: Vec<Span>,
    uses_component: bool,
    out: Vec<Finding>,
}

impl Elements<'_> {
    fn checked(&mut self, name: &JSXElementName) -> bool {
        let (project, file) = (self.project, self.file);
        *self.checked.entry(name.span()).or_insert_with(|| {
            project
                .tag_file(file, name)
                .is_some_and(|f| project.is_checked_use(f))
        })
    }
}

impl<'a> Visit<'a> for Elements<'_> {
    fn visit_astro_script(&mut self, _it: &AstroScript<'a>) {}

    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        let name = &element.name;
        if is_div(name) {
            self.divs.push(name.span());
        } else {
            if self.checked(name) {
                self.uses_component = true;
            }
            for attribute in &element.attributes {
                let JSXAttributeItem::Attribute(attribute) = attribute else {
                    continue;
                };
                let JSXAttributeName::Identifier(id) = &attribute.name else {
                    continue;
                };
                let is_fill = id.name == "fill";
                if !is_fill && id.name != "surface" {
                    continue;
                }
                let Some(value) = attribute.value.as_ref().and_then(value_literal) else {
                    continue;
                };
                if !self.checked(name) {
                    continue;
                }
                let found = if is_fill {
                    fill_finding(self.project, &value)
                } else {
                    surface_finding(self.project, &value)
                };
                let Some((code, message, help)) = found else {
                    continue;
                };
                let (line, column) = self.file.position(attribute.span.start as usize);
                let (end_line, end_column) = self.file.position(attribute.span.end as usize);
                self.out.push(Finding {
                    file: self.rel.to_string(),
                    line,
                    column,
                    end_line,
                    end_column,
                    severity: "error",
                    code,
                    message,
                    help,
                });
            }
        }
        walk::walk_jsx_opening_element(self, element);
    }
}

fn element_findings(project: &Project, rel: &str, file: &'static Parsed) -> Vec<Finding> {
    let mut elements = Elements {
        project,
        rel,
        file,
        checked: HashMap::new(),
        divs: vec![],
        uses_component: false,
        out: vec![],
    };
    walk_file(&mut elements, file);
    let mut out = elements.out;
    if elements.uses_component {
        for span in elements.divs {
            let (line, column) = file.position(span.start as usize);
            let (end_line, end_column) = file.position(span.end as usize);
            out.push(Finding {
                file: rel.to_string(),
                line,
                column,
                end_line,
                end_column,
                severity: "warning",
                code: "rafters(freehand-div)",
                message: "a div laid by hand around rafters components".to_string(),
                help: Some(FREEHAND_HELP.to_string()),
            });
        }
    }
    out
}

// ---- The program's files: tsconfig include and exclude, as TypeScript expands them.

fn read_jsonc(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                out.extend(chars.next());
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            out.push(c);
        } else if c == '/' && chars.peek() == Some(&'/') {
            for n in chars.by_ref() {
                if n == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut last = ' ';
            for n in chars.by_ref() {
                if last == '*' && n == '/' {
                    break;
                }
                last = n;
            }
        } else {
            out.push(c);
        }
    }
    serde_json::from_str(&out).ok()
}

fn glob_match(pattern: &[&str], path: &[&str]) -> bool {
    match (pattern.first(), path.first()) {
        (None, None) => true,
        (Some(&"**"), _) => {
            glob_match(&pattern[1..], path) || (!path.is_empty() && glob_match(pattern, &path[1..]))
        }
        (Some(p), Some(s)) => {
            segment_match(p.as_bytes(), s.as_bytes()) && glob_match(&pattern[1..], &path[1..])
        }
        _ => false,
    }
}

fn segment_match(p: &[u8], s: &[u8]) -> bool {
    match (p.first(), s.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            segment_match(&p[1..], s) || (!s.is_empty() && segment_match(p, &s[1..]))
        }
        (Some(b'?'), Some(_)) => segment_match(&p[1..], &s[1..]),
        (Some(a), Some(b)) => a == b && segment_match(&p[1..], &s[1..]),
        _ => false,
    }
}

const CHECKED: [&str; 3] = [".tsx", ".astro", ".classes.ts"];

struct Roots {
    include: Vec<Vec<String>>,
    exclude: Vec<Vec<String>>,
}

impl Roots {
    fn read(root: &Path) -> Roots {
        let config = read_jsonc(&root.join("tsconfig.json")).unwrap_or_default();
        let split = |p: &str| -> Vec<String> {
            p.trim_start_matches("./")
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .map(String::from)
                .collect()
        };
        let list = |key: &str| {
            config[key].as_array().map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(split)
                    .collect::<Vec<_>>()
            })
        };
        Roots {
            include: list("include").unwrap_or_else(|| vec![split("**/*")]),
            exclude: list("exclude").unwrap_or_else(|| vec![split("node_modules")]),
        }
    }

    fn holds(&self, rel: &str) -> bool {
        let parts: Vec<&str> = rel.split('/').collect();
        // A wildcard never reaches into node_modules or a folder or file whose name starts with a dot.
        let implicit = parts
            .iter()
            .any(|p| p.starts_with('.') || *p == "node_modules");
        let included = self.include.iter().any(|pattern| {
            let pattern: Vec<&str> = pattern.iter().map(String::as_str).collect();
            let wild = pattern.iter().any(|p| p.contains('*') || p.contains('?'));
            if wild && implicit {
                return false;
            }
            glob_match(&pattern, &parts)
                || (!wild && rel.starts_with(&format!("{}/", pattern.join("/"))))
        });
        let excluded = self.exclude.iter().any(|pattern| {
            let pattern: Vec<&str> = pattern.iter().map(String::as_str).collect();
            (1..=parts.len()).any(|n| glob_match(&pattern, &parts[..n]))
        });
        included && !excluded
    }
}

fn walk_dir(root: &Path, dir: &Path, roots: &Roots, into: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() || (kind.is_symlink() && path.is_dir()) {
            walk_dir(root, &path, roots, into);
        } else if CHECKED.iter().any(|ext| name.ends_with(ext)) {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if roots.holds(&rel) {
                into.push(rel);
            }
        }
    }
}

// ---- The call.

fn check(project: &Project, files: &[String]) -> Vec<Finding> {
    let mut found = vec![];
    for rel in files {
        let Some(file) = project.file(&project.root.join(rel)) else {
            eprintln!("{rel} could not be parsed, so it was not checked");
            continue;
        };
        if project.is_exempt(file) {
            continue;
        }
        found.extend(bracket_values(project, rel, file));
        found.extend(element_findings(project, rel, file));
    }
    found.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.column.cmp(&b.column))
    });
    found
}

fn report(findings: &[Finding]) -> String {
    let mut out = String::new();
    for f in findings {
        out.push_str(&format!(
            "{}:{}:{} {} {} {}\n",
            f.file, f.line, f.column, f.severity, f.code, f.message
        ));
        if let Some(help) = &f.help {
            out.push_str(&format!("  help: {help}\n"));
        }
    }
    let n = findings.len();
    out.push_str(&if n == 1 {
        "1 problem\n".to_string()
    } else {
        format!("{n} problems\n")
    });
    out
}

fn jsonl(findings: &[Finding]) -> String {
    findings
        .iter()
        .map(|f| {
            let mut v = serde_json::json!({
                "file": f.file, "line": f.line, "column": f.column, "endLine": f.end_line,
                "endColumn": f.end_column, "severity": f.severity, "code": f.code, "message": f.message,
            });
            if let Some(help) = &f.help {
                v["help"] = help.clone().into();
            }
            format!("{v}\n")
        })
        .collect()
}

fn resolved_links(file: &Path) -> PathBuf {
    if file.exists() {
        return real(file);
    }
    match (file.parent(), file.file_name()) {
        (Some(parent), Some(name)) if parent != file => resolved_links(parent).join(name),
        _ => file.to_path_buf(),
    }
}

fn run() -> Result<i32, String> {
    let mut args = std::env::args().skip(1);
    let (mut root, mut stdin_path, mut as_jsonl) = (None, None, false);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stdin" => {
                stdin_path = Some(
                    args.next()
                        .ok_or("usage: t18-oxc-check [root] [--stdin <path>] [--jsonl]")?,
                )
            }
            "--jsonl" => as_jsonl = true,
            _ if root.is_none() => root = Some(arg),
            _ => return Err("usage: t18-oxc-check [root] [--stdin <path>] [--jsonl]".into()),
        }
    }
    let root = real(&PathBuf::from(root.unwrap_or_else(|| ".".into())));
    let roots = Roots::read(&root);
    let (unsaved, files) = match stdin_path {
        Some(p) => {
            let absolute =
                resolved_links(&std::env::current_dir().map_err(|e| e.to_string())?.join(p));
            let rel = absolute
                .strip_prefix(&root)
                .map_err(|_| "the --stdin path is outside the root")?
                .to_string_lossy()
                .replace('\\', "/");
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            let files = if !roots.holds(&rel) {
                eprintln!("{rel} is not in the tsconfig's files, so it was not checked");
                vec![]
            } else if CHECKED.iter().any(|ext| rel.ends_with(ext)) {
                vec![rel]
            } else {
                vec![]
            };
            (Some((absolute, text)), files)
        }
        None => {
            let mut files = vec![];
            walk_dir(&root, &root, &roots, &mut files);
            files.sort();
            (None, files)
        }
    };
    let project = Project::open(&root, unsaved)?;
    let findings = check(&project, &files);
    if std::env::var_os("T18_STATS").is_some() {
        eprintln!(
            "checked {} files, parsed {} files",
            files.len(),
            project.cache.borrow().len()
        );
    }
    print!(
        "{}",
        if as_jsonl {
            jsonl(&findings)
        } else {
            report(&findings)
        }
    );
    Ok(i32::from(findings.iter().any(|f| f.severity == "error")))
}

fn main() {
    let code = run().unwrap_or_else(|e| {
        eprintln!("{e}");
        2
    });
    std::process::exit(code);
}
