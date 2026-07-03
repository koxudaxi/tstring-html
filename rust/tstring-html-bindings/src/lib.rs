use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyIterator, PyModule};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tstring_html::{
    AttributeLike, CompiledHtmlTemplate, Document, FormatOptions, Node, RuntimeContext,
    RuntimeValue, prepare_template as prepare_html_template, rebind_document_interpolations,
    render_attributes_fragment, render_html as render_html_compiled,
};
use tstring_syntax::{BackendError, ErrorKind, TemplateInput, TemplateInterpolation};
use tstring_thtml::{CompiledThtmlTemplate, prepare_template as prepare_thtml_template};

create_exception!(tstring_html_bindings, TemplateError, PyException);
create_exception!(tstring_html_bindings, TemplateParseError, TemplateError);
create_exception!(tstring_html_bindings, TemplateSemanticError, TemplateError);
create_exception!(tstring_html_bindings, TemplateRuntimeError, TemplateError);

const PARSE_CACHE_CAPACITY: usize = 256;
const CONTRACT_VERSION: u32 = 2;
const REGISTRY_TYPE_ERROR: &str = "registry= must be mapping-like.";
const CONTRACT_SYMBOLS: &[&str] = &[
    "TemplateError",
    "TemplateParseError",
    "TemplateSemanticError",
    "TemplateRuntimeError",
    "Fragment",
    "RawHtml",
    "Renderable",
    "check_html_template",
    "format_html_template",
    "compile_html_template",
    "render_html_template",
    "render_html_fragment",
    "check_thtml_template",
    "format_thtml_template",
    "compile_thtml_template",
    "render_thtml_template",
];

type CacheKey = (String, String, Vec<String>);

struct ParseCache<T> {
    capacity: usize,
    state: Mutex<ParseCacheState<T>>,
}

struct ParseCacheState<T> {
    entries: HashMap<CacheKey, Arc<T>>,
    order: VecDeque<CacheKey>,
}

impl<T> ParseCache<T> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            state: Mutex::new(ParseCacheState {
                entries: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    fn get_or_try_insert_with<E, F>(&self, key: &CacheKey, build: F) -> Result<Arc<T>, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if let Some(value) = self.get(key) {
            return Ok(value);
        }
        let parsed = Arc::new(build()?);
        let key = key.clone();
        let mut state = self.lock_state();
        if let Some(existing) = state.entries.get(&key).cloned() {
            Self::touch_key(&mut state, &key);
            return Ok(existing);
        }
        self.insert_locked(&mut state, key, Arc::clone(&parsed));
        Ok(parsed)
    }

    fn get(&self, key: &CacheKey) -> Option<Arc<T>> {
        let key = key.clone();
        let mut state = self.lock_state();
        let value = state.entries.get(&key).cloned();
        if value.is_some() {
            Self::touch_key(&mut state, &key);
        }
        value
    }

    fn insert_locked(&self, state: &mut ParseCacheState<T>, key: CacheKey, value: Arc<T>) {
        if state.entries.len() == self.capacity {
            while let Some(oldest) = state.order.pop_front() {
                if state.entries.remove(&oldest).is_some() {
                    break;
                }
            }
        }
        state.order.push_back(key.clone());
        state.entries.insert(key, value);
    }

    fn touch_key(state: &mut ParseCacheState<T>, key: &CacheKey) {
        if let Some(index) = state.order.iter().position(|entry| entry == key) {
            state.order.remove(index);
        }
        state.order.push_back(key.clone());
    }

    fn lock_state(&self) -> MutexGuard<'_, ParseCacheState<T>> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

fn html_parse_cache() -> &'static ParseCache<Document> {
    static CACHE: OnceLock<ParseCache<Document>> = OnceLock::new();
    CACHE.get_or_init(|| ParseCache::new(PARSE_CACHE_CAPACITY))
}

fn thtml_parse_cache() -> &'static ParseCache<Document> {
    static CACHE: OnceLock<ParseCache<Document>> = OnceLock::new();
    CACHE.get_or_init(|| ParseCache::new(PARSE_CACHE_CAPACITY))
}

#[pyclass(module = "tstring_html_bindings.tstring_html_bindings")]
struct Fragment {
    #[pyo3(get)]
    items: Vec<Py<PyAny>>,
}

#[pymethods]
impl Fragment {
    #[new]
    fn new(items: Vec<Py<PyAny>>) -> Self {
        Self { items }
    }
}

#[pyclass(module = "tstring_html_bindings.tstring_html_bindings")]
#[derive(Clone)]
struct RawHtml {
    #[pyo3(get)]
    value: String,
}

#[pymethods]
impl RawHtml {
    #[new]
    fn new(value: String) -> Self {
        Self { value }
    }

    fn __str__(&self) -> String {
        self.value.clone()
    }
}

#[pyclass(
    module = "tstring_html_bindings.tstring_html_bindings",
    name = "CompiledHtmlTemplate"
)]
struct PyCompiledHtmlTemplate {
    compiled: Arc<CompiledHtmlTemplate>,
}

#[pyclass(
    module = "tstring_html_bindings.tstring_html_bindings",
    name = "CompiledThtmlTemplate"
)]
struct PyCompiledThtmlTemplate {
    compiled: Arc<CompiledThtmlTemplate>,
}

#[pymethods]
impl PyCompiledHtmlTemplate {
    fn render(&self, py: Python<'_>, values: Vec<Py<PyAny>>) -> PyResult<String> {
        let context = runtime_context_from_values(py, &values)?;
        render_html_compiled(self.compiled.as_ref(), &context).map_err(backend_error_to_py)
    }

    fn render_fragment(&self, py: Python<'_>, values: Vec<Py<PyAny>>) -> PyResult<String> {
        let context = runtime_context_from_values(py, &values)?;
        Ok(
            tstring_html::render_fragment(self.compiled.as_ref(), &context)
                .map_err(backend_error_to_py)?
                .html,
        )
    }

    fn __repr__(&self) -> String {
        "CompiledHtmlTemplate()".to_string()
    }
}

#[pymethods]
impl PyCompiledThtmlTemplate {
    #[pyo3(signature = (values, globals = None, locals = None, registry = None))]
    fn render(
        &self,
        py: Python<'_>,
        values: Vec<Py<PyAny>>,
        globals: Option<&Bound<'_, PyDict>>,
        locals: Option<&Bound<'_, PyDict>>,
        registry: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<String> {
        let (globals, locals) = normalize_scope_inputs(
            py,
            globals,
            locals,
            registry,
            "CompiledThtmlTemplate.render",
        )?;
        let context = runtime_context_from_values(py, &values)?;
        render_thtml_document(
            py,
            self.compiled.document(),
            &context,
            globals.as_ref(),
            locals.as_ref(),
        )
    }

    fn __repr__(&self) -> String {
        "CompiledThtmlTemplate()".to_string()
    }
}

struct BoundTemplate {
    input: TemplateInput,
    strings: Vec<String>,
    values: Vec<Py<PyAny>>,
}

impl BoundTemplate {
    fn cache_key_strings(&self) -> &[String] {
        &self.strings
    }
}

fn template_cache_key(bound: &BoundTemplate, backend: &str) -> CacheKey {
    (
        backend.to_string(),
        "parse_validated".to_string(),
        bound.cache_key_strings().to_vec(),
    )
}

fn backend_error_to_py(err: BackendError) -> PyErr {
    match err.kind {
        ErrorKind::Parse => TemplateParseError::new_err(err.message),
        ErrorKind::Semantic | ErrorKind::Unrepresentable => {
            TemplateSemanticError::new_err(err.message)
        }
    }
}

fn runtime_error_to_py(message: impl Into<String>) -> PyErr {
    TemplateRuntimeError::new_err(message.into())
}

fn ensure_template(py: Python<'_>, template: &Bound<'_, PyAny>, api_name: &str) -> PyResult<()> {
    let templatelib = py.import("string.templatelib")?;
    let template_type = templatelib.getattr("Template")?;
    if template.is_instance(&template_type)? {
        return Ok(());
    }
    Err(PyTypeError::new_err(format!(
        "{api_name} requires a PEP 750 Template object. Got {} instead.",
        template.get_type().name()?
    )))
}

fn extract_template(
    py: Python<'_>,
    template: &Bound<'_, PyAny>,
    api_name: &str,
) -> PyResult<BoundTemplate> {
    ensure_template(py, template, api_name)?;
    let strings: Vec<String> = template.getattr("strings")?.extract()?;
    let interpolation_seq = template.getattr("interpolations")?;
    let iterator = interpolation_seq.try_iter()?;
    let mut interpolations = Vec::new();
    let mut values = Vec::new();
    for (interpolation_index, item) in iterator.enumerate() {
        let interpolation = item?;
        let expression = interpolation
            .getattr("expression")?
            .extract::<Option<String>>()?
            .unwrap_or_default();
        let conversion = interpolation
            .getattr("conversion")?
            .extract::<Option<String>>()?;
        let format_spec = interpolation
            .getattr("format_spec")?
            .extract::<Option<String>>()?
            .unwrap_or_default();
        let raw_source = build_raw_source(&expression, conversion.as_deref(), &format_spec);
        values.push(interpolation.getattr("value")?.unbind());
        interpolations.push(TemplateInterpolation {
            expression,
            conversion,
            format_spec,
            interpolation_index,
            raw_source: Some(raw_source),
        });
    }
    Ok(BoundTemplate {
        input: TemplateInput::from_parts(strings.clone(), interpolations),
        strings,
        values,
    })
}

fn build_raw_source(expression: &str, conversion: Option<&str>, format_spec: &str) -> String {
    let mut raw = String::from("{");
    raw.push_str(expression);
    if let Some(conversion) = conversion {
        raw.push('!');
        raw.push_str(conversion);
    }
    if !format_spec.is_empty() {
        raw.push(':');
        raw.push_str(format_spec);
    }
    raw.push('}');
    raw
}

fn prepared_html_document(bound: &BoundTemplate) -> PyResult<Document> {
    let cached = html_parse_cache()
        .get_or_try_insert_with(&template_cache_key(bound, "html"), || {
            prepare_html_template(&bound.input)
        })
        .map_err(backend_error_to_py)?;
    let mut document = (*cached).clone();
    rebind_document_interpolations(&mut document, &bound.input);
    Ok(document)
}

fn prepared_thtml_document(bound: &BoundTemplate) -> PyResult<Document> {
    let cached = thtml_parse_cache()
        .get_or_try_insert_with(&template_cache_key(bound, "thtml"), || {
            prepare_thtml_template(&bound.input)
        })
        .map_err(backend_error_to_py)?;
    let mut document = (*cached).clone();
    rebind_document_interpolations(&mut document, &bound.input);
    Ok(document)
}

fn compile_cached_html(bound: &BoundTemplate) -> PyResult<Arc<CompiledHtmlTemplate>> {
    Ok(Arc::new(CompiledHtmlTemplate::from_document(
        prepared_html_document(bound)?,
    )))
}

fn compile_cached_thtml(bound: &BoundTemplate) -> PyResult<Arc<CompiledThtmlTemplate>> {
    Ok(Arc::new(CompiledThtmlTemplate::from_document(
        prepared_thtml_document(bound)?,
    )))
}

fn runtime_value_from_py(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<RuntimeValue> {
    if value.is_none() {
        return Ok(RuntimeValue::Null);
    }
    if let Ok(marker) = value.getattr("__tstring_renderable__") {
        if marker.is_truthy()? {
            let rendered = value.call_method0("render")?;
            let rendered: String = rendered
                .extract()
                .map_err(|_| runtime_error_to_py("Renderable.render() must return a string."))?;
            return Ok(RuntimeValue::RawHtml(rendered));
        }
    }
    if let Ok(raw) = value.extract::<bool>() {
        return Ok(RuntimeValue::Bool(raw));
    }
    if let Ok(raw) = value.extract::<i64>() {
        return Ok(RuntimeValue::Int(raw));
    }
    if let Ok(raw) = value.extract::<f64>() {
        return Ok(RuntimeValue::Float(raw));
    }
    if let Ok(raw) = value.extract::<String>() {
        return Ok(RuntimeValue::String(raw));
    }
    if let Ok(raw) = value.extract::<PyRef<'_, RawHtml>>() {
        return Ok(RuntimeValue::RawHtml(raw.value.clone()));
    }
    if let Ok(fragment) = value.extract::<PyRef<'_, Fragment>>() {
        let mut values = Vec::new();
        for item in &fragment.items {
            values.push(runtime_value_from_py(py, item.bind(py))?);
        }
        return Ok(RuntimeValue::Fragment(values));
    }
    if let Ok(dict) = value.cast::<PyDict>() {
        let mut entries = Vec::new();
        for (key, value) in dict.iter() {
            entries.push((key.extract::<String>()?, runtime_value_from_py(py, &value)?));
        }
        return Ok(RuntimeValue::Attributes(entries));
    }
    if let Ok(iter) = PyIterator::from_object(value) {
        let mut entries = Vec::new();
        for item in iter {
            entries.push(runtime_value_from_py(py, &item?)?);
        }
        return Ok(RuntimeValue::Sequence(entries));
    }
    Ok(RuntimeValue::String(value.str()?.extract()?))
}

fn runtime_context_from_bound(py: Python<'_>, bound: &BoundTemplate) -> PyResult<RuntimeContext> {
    runtime_context_from_values(py, &bound.values)
}

fn runtime_context_from_values(py: Python<'_>, values: &[Py<PyAny>]) -> PyResult<RuntimeContext> {
    let mut runtime_values = Vec::with_capacity(values.len());
    for value in values {
        runtime_values.push(runtime_value_from_py(py, value.bind(py))?);
    }
    Ok(RuntimeContext {
        values: runtime_values,
    })
}

fn registry_to_scope_dict<'py>(
    py: Python<'py>,
    registry: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let builtins = py.import("builtins")?;
    let dict = builtins
        .getattr("dict")?
        .call1((registry,))
        .map_err(|_| PyTypeError::new_err(REGISTRY_TYPE_ERROR))?;
    dict.cast_into::<PyDict>()
        .map_err(|_| PyTypeError::new_err(REGISTRY_TYPE_ERROR))
}

fn normalize_scope_inputs<'py>(
    py: Python<'py>,
    globals: Option<&Bound<'py, PyDict>>,
    locals: Option<&Bound<'py, PyDict>>,
    registry: Option<&Bound<'py, PyAny>>,
    api_name: &str,
) -> PyResult<(Option<Bound<'py, PyDict>>, Option<Bound<'py, PyDict>>)> {
    if registry.is_some() && (globals.is_some() || locals.is_some()) {
        return Err(PyTypeError::new_err(format!(
            "{api_name} does not allow combining registry= with globals= or locals=."
        )));
    }

    if let Some(registry) = registry {
        return Ok((
            Some(registry_to_scope_dict(py, registry)?),
            Some(PyDict::new(py)),
        ));
    }

    Ok((globals.cloned(), locals.cloned()))
}

fn render_thtml_document(
    py: Python<'_>,
    document: &Document,
    context: &RuntimeContext,
    globals: Option<&Bound<'_, PyDict>>,
    locals: Option<&Bound<'_, PyDict>>,
) -> PyResult<String> {
    let mut out = String::new();
    for node in &document.children {
        render_thtml_node(py, node, context, globals, locals, &mut out)?;
    }
    Ok(out)
}

fn normalized_children_value(
    py: Python<'_>,
    children: &[Node],
    context: &RuntimeContext,
    globals: Option<&Bound<'_, PyDict>>,
    locals: Option<&Bound<'_, PyDict>>,
) -> PyResult<Py<PyAny>> {
    let mut values = Vec::with_capacity(children.len());
    for child in children {
        let value = evaluate_thtml_child_value(py, child, context, globals, locals)?;
        if let Some(value) = normalize_component_child(py, &value, true)? {
            values.push(value);
        }
    }

    match values.len() {
        0 => Ok(py.None()),
        1 => Ok(values.pop().expect("single child")),
        _ => Ok(Py::new(py, Fragment { items: values })?.into_any()),
    }
}

fn normalize_component_child(
    py: Python<'_>,
    value: &RuntimeValue,
    preserve_container: bool,
) -> PyResult<Option<Py<PyAny>>> {
    match value {
        RuntimeValue::Null => Ok(None),
        RuntimeValue::Sequence(values) => {
            let items = normalize_sequence_items(py, values)?;
            if preserve_container {
                Ok(Some(items.into_pyobject(py)?.unbind().into_any()))
            } else {
                Ok(Some(items.into_pyobject(py)?.unbind().into_any()))
            }
        }
        RuntimeValue::Fragment(values) => {
            let items = normalize_sequence_items(py, values)?;
            if preserve_container {
                Ok(Some(Py::new(py, Fragment { items })?.into_any()))
            } else {
                Ok(Some(items.into_pyobject(py)?.unbind().into_any()))
            }
        }
        _ => Ok(Some(python_from_runtime_value(py, value)?)),
    }
}

fn normalize_sequence_items(py: Python<'_>, values: &[RuntimeValue]) -> PyResult<Vec<Py<PyAny>>> {
    let mut items = Vec::new();
    for value in values {
        match value {
            RuntimeValue::Null => {}
            RuntimeValue::Sequence(nested) | RuntimeValue::Fragment(nested) => {
                items.extend(normalize_sequence_items(py, nested)?);
            }
            _ => items.push(python_from_runtime_value(py, value)?),
        }
    }
    Ok(items)
}

fn evaluate_thtml_child_value(
    py: Python<'_>,
    node: &Node,
    context: &RuntimeContext,
    globals: Option<&Bound<'_, PyDict>>,
    locals: Option<&Bound<'_, PyDict>>,
) -> PyResult<RuntimeValue> {
    match node {
        Node::Text(text) => Ok(RuntimeValue::String(text.value.clone())),
        Node::Interpolation(interpolation) => context
            .values
            .get(interpolation.interpolation_index)
            .cloned()
            .ok_or_else(|| runtime_error_to_py("Missing runtime value for interpolation.")),
        Node::Fragment(fragment) => {
            let mut values = Vec::with_capacity(fragment.children.len());
            for child in &fragment.children {
                values.push(evaluate_thtml_child_value(
                    py, child, context, globals, locals,
                )?);
            }
            Ok(RuntimeValue::Fragment(values))
        }
        Node::Element(_)
        | Node::RawTextElement(_)
        | Node::Comment(_)
        | Node::Doctype(_)
        | Node::ComponentTag(_) => {
            let mut out = String::new();
            render_thtml_node(py, node, context, globals, locals, &mut out)?;
            Ok(RuntimeValue::RawHtml(out))
        }
    }
}

fn render_thtml_node(
    py: Python<'_>,
    node: &Node,
    context: &RuntimeContext,
    globals: Option<&Bound<'_, PyDict>>,
    locals: Option<&Bound<'_, PyDict>>,
    out: &mut String,
) -> PyResult<()> {
    match node {
        Node::ComponentTag(component) => {
            let callable = resolve_component(py, &component.name, globals, locals)?;
            let kwargs = PyDict::new(py);
            let children =
                normalized_children_value(py, &component.children, context, globals, locals)?;
            kwargs.set_item("children", children)?;
            for attribute in &component.attributes {
                match attribute {
                    AttributeLike::Attribute(attribute) => {
                        if let Some(value) = &attribute.value {
                            kwargs.set_item(
                                &attribute.name,
                                render_attribute_value_for_component(py, value, context)?,
                            )?;
                        } else {
                            kwargs.set_item(&attribute.name, true)?;
                        }
                    }
                    AttributeLike::SpreadAttribute(attribute) => {
                        let index = attribute.interpolation.interpolation_index;
                        let Some(value) = context.values.get(index) else {
                            return Err(runtime_error_to_py(
                                "Missing runtime value for spread attribute.",
                            ));
                        };
                        if let RuntimeValue::Attributes(entries) = value {
                            for (key, value) in entries {
                                if !tstring_html::is_valid_html_attribute_name(key) {
                                    return Err(runtime_error_to_py(format!(
                                        "Spread attribute name {key:?} is not a valid HTML attribute name."
                                    )));
                                }
                                kwargs.set_item(key, python_from_runtime_value(py, value)?)?;
                            }
                        } else {
                            return Err(runtime_error_to_py(
                                "Spread attributes require a mapping-like value.",
                            ));
                        }
                    }
                }
            }
            let rendered = callable.call((), Some(&kwargs))?;
            let rendered = runtime_value_from_py(py, &rendered)?;
            tstring_html::render_child_value(&rendered, out).map_err(backend_error_to_py)?;
        }
        Node::Element(element) => {
            out.push('<');
            out.push_str(&element.name);
            out.push_str(
                &render_attributes_fragment(&element.attributes, context)
                    .map_err(backend_error_to_py)?,
            );
            if element.self_closing {
                out.push_str(" />");
                return Ok(());
            }
            out.push('>');
            for child in &element.children {
                render_thtml_node(py, child, context, globals, locals, out)?;
            }
            out.push_str("</");
            out.push_str(&element.name);
            out.push('>');
        }
        Node::RawTextElement(element) => {
            out.push('<');
            out.push_str(&element.name);
            out.push_str(
                &render_attributes_fragment(&element.attributes, context)
                    .map_err(backend_error_to_py)?,
            );
            out.push('>');
            for child in &element.children {
                match child {
                    Node::Text(text) => out.push_str(&text.value),
                    Node::Interpolation(interpolation)
                        if element.name.eq_ignore_ascii_case("title") =>
                    {
                        let Some(value) = context.values.get(interpolation.interpolation_index)
                        else {
                            return Err(runtime_error_to_py(
                                "Missing runtime value for interpolation.",
                            ));
                        };
                        render_escaped_text_value(value, out).map_err(backend_error_to_py)?;
                    }
                    _ => {
                        return Err(runtime_error_to_py(
                            "Invalid raw-text content in T-HTML render path.",
                        ));
                    }
                }
            }
            out.push_str("</");
            out.push_str(&element.name);
            out.push('>');
        }
        Node::Text(text) => out.push_str(&text.value),
        Node::Interpolation(interpolation) => {
            let Some(value) = context.values.get(interpolation.interpolation_index) else {
                return Err(runtime_error_to_py(
                    "Missing runtime value for interpolation.",
                ));
            };
            tstring_html::render_child_value(value, out).map_err(backend_error_to_py)?;
        }
        Node::Comment(comment) => {
            out.push_str("<!--");
            out.push_str(&comment.value);
            out.push_str("-->");
        }
        Node::Doctype(doctype) => {
            out.push_str("<!");
            out.push_str(&doctype.value);
            out.push('>');
        }
        Node::Fragment(fragment) => {
            for child in &fragment.children {
                render_thtml_node(py, child, context, globals, locals, out)?;
            }
        }
    }
    Ok(())
}

fn render_escaped_text_value(value: &RuntimeValue, out: &mut String) -> Result<(), BackendError> {
    match value {
        RuntimeValue::Null => {}
        RuntimeValue::Bool(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::Int(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::Float(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::String(value) => out.push_str(&escape_html_text(value)),
        RuntimeValue::RawHtml(value) => out.push_str(&escape_html_text(value)),
        RuntimeValue::Fragment(values) | RuntimeValue::Sequence(values) => {
            for value in values {
                render_escaped_text_value(value, out)?;
            }
        }
        RuntimeValue::Attributes(_) => {
            return Err(tstring_html::runtime_error(
                "html.runtime.child_type",
                "Mapping-like values cannot be rendered as children.",
                None,
            ));
        }
    }
    Ok(())
}

fn escape_html_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn render_attribute_value_for_component(
    py: Python<'_>,
    value: &tstring_html::AttributeValue,
    context: &RuntimeContext,
) -> PyResult<Py<PyAny>> {
    let mut rendered = String::new();
    for part in &value.parts {
        match part {
            tstring_html::ValuePart::Text(text) => rendered.push_str(text),
            tstring_html::ValuePart::Interpolation(interpolation) => {
                let Some(value) = context.values.get(interpolation.interpolation_index) else {
                    return Err(runtime_error_to_py(
                        "Missing runtime value for component attribute.",
                    ));
                };
                rendered.push_str(
                    &tstring_html::stringify_runtime_value(value).map_err(backend_error_to_py)?,
                );
            }
        }
    }
    Ok(rendered.into_pyobject(py)?.unbind().into_any())
}

fn python_from_runtime_value(py: Python<'_>, value: &RuntimeValue) -> PyResult<Py<PyAny>> {
    match value {
        RuntimeValue::Null => Ok(py.None()),
        RuntimeValue::Bool(value) => Ok(PyBool::new(py, *value).to_owned().unbind().into_any()),
        RuntimeValue::Int(value) => Ok((*value).into_pyobject(py)?.unbind().into_any()),
        RuntimeValue::Float(value) => Ok((*value).into_pyobject(py)?.unbind().into_any()),
        RuntimeValue::String(value) => Ok(value.clone().into_pyobject(py)?.unbind().into_any()),
        RuntimeValue::RawHtml(value) => Ok(Py::new(
            py,
            RawHtml {
                value: value.clone(),
            },
        )?
        .into_any()),
        RuntimeValue::Fragment(values) => {
            let mut items = Vec::new();
            for value in values {
                items.push(python_from_runtime_value(py, value)?);
            }
            Ok(Py::new(py, Fragment { items })?.into_any())
        }
        RuntimeValue::Sequence(values) => {
            let mut items = Vec::new();
            for value in values {
                items.push(python_from_runtime_value(py, value)?);
            }
            Ok(items.into_pyobject(py)?.unbind().into_any())
        }
        RuntimeValue::Attributes(entries) => {
            let dict = PyDict::new(py);
            for (key, value) in entries {
                dict.set_item(key, python_from_runtime_value(py, value)?)?;
            }
            Ok(dict.unbind().into_any())
        }
    }
}

fn resolve_component<'py>(
    py: Python<'py>,
    name: &str,
    globals: Option<&Bound<'py, PyDict>>,
    locals: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let globals = match globals {
        Some(globals) => globals.clone(),
        None => default_scope_dict(py, true)?,
    };
    let locals = match locals {
        Some(locals) => locals.clone(),
        None => default_scope_dict(py, false)?,
    };

    if let Ok(Some(value)) = locals.get_item(name) {
        if value.is_callable() {
            return Ok(value);
        }
        return Err(runtime_error_to_py(format!(
            "Resolved component '{name}' is not callable."
        )));
    }
    if let Ok(Some(value)) = globals.get_item(name) {
        if value.is_callable() {
            return Ok(value);
        }
        return Err(runtime_error_to_py(format!(
            "Resolved component '{name}' is not callable."
        )));
    }
    Err(runtime_error_to_py(format!(
        "Unknown component '{name}'. Pass registry=, globals=, or locals= explicitly."
    )))
}

fn default_scope_dict<'py>(py: Python<'py>, globals: bool) -> PyResult<Bound<'py, PyDict>> {
    let sys = py.import("sys")?;
    let frame = sys
        .getattr("_getframe")?
        .call1((1,)) // immediate caller only
        .map_err(|_| {
            runtime_error_to_py(
                "Caller-frame inspection failed. Pass registry=, globals=, or locals= explicitly.",
            )
        })?;
    let dict = if globals {
        frame.getattr("f_globals")?
    } else {
        frame.getattr("f_locals")?
    };
    dict.cast_into::<PyDict>().map_err(|_| {
        runtime_error_to_py(
            "Caller-frame inspection failed. Pass registry=, globals=, or locals= explicitly.",
        )
    })
}

#[pyfunction]
fn check_html_template(py: Python<'_>, template: &Bound<'_, PyAny>) -> PyResult<()> {
    let bound = extract_template(py, template, "check_html_template")?;
    tstring_html::check_template(&bound.input).map_err(backend_error_to_py)
}

#[pyfunction(signature = (template, *, line_length = 80))]
fn format_html_template(
    py: Python<'_>,
    template: &Bound<'_, PyAny>,
    line_length: usize,
) -> PyResult<String> {
    let bound = extract_template(py, template, "format_html_template")?;
    tstring_html::format_template_with_options(&bound.input, &FormatOptions { line_length })
        .map_err(backend_error_to_py)
}

#[pyfunction]
fn compile_html_template(
    py: Python<'_>,
    template: &Bound<'_, PyAny>,
) -> PyResult<PyCompiledHtmlTemplate> {
    let bound = extract_template(py, template, "compile_html_template")?;
    Ok(PyCompiledHtmlTemplate {
        compiled: compile_cached_html(&bound)?,
    })
}

#[pyfunction]
fn render_html_template(py: Python<'_>, template: &Bound<'_, PyAny>) -> PyResult<String> {
    let bound = extract_template(py, template, "render_html_template")?;
    let compiled = compile_cached_html(&bound)?;
    let context = runtime_context_from_bound(py, &bound)?;
    render_html_compiled(compiled.as_ref(), &context).map_err(backend_error_to_py)
}

#[pyfunction]
fn render_html_fragment(py: Python<'_>, template: &Bound<'_, PyAny>) -> PyResult<String> {
    let bound = extract_template(py, template, "render_html_fragment")?;
    let compiled = compile_cached_html(&bound)?;
    let context = runtime_context_from_bound(py, &bound)?;
    Ok(tstring_html::render_fragment(compiled.as_ref(), &context)
        .map_err(backend_error_to_py)?
        .html)
}

#[pyfunction]
fn check_thtml_template(py: Python<'_>, template: &Bound<'_, PyAny>) -> PyResult<()> {
    let bound = extract_template(py, template, "check_thtml_template")?;
    tstring_thtml::check_template(&bound.input).map_err(backend_error_to_py)
}

#[pyfunction(signature = (template, *, line_length = 80))]
fn format_thtml_template(
    py: Python<'_>,
    template: &Bound<'_, PyAny>,
    line_length: usize,
) -> PyResult<String> {
    let bound = extract_template(py, template, "format_thtml_template")?;
    tstring_thtml::format_template_with_options(&bound.input, &FormatOptions { line_length })
        .map_err(backend_error_to_py)
}

#[pyfunction]
fn compile_thtml_template(
    py: Python<'_>,
    template: &Bound<'_, PyAny>,
) -> PyResult<PyCompiledThtmlTemplate> {
    let bound = extract_template(py, template, "compile_thtml_template")?;
    Ok(PyCompiledThtmlTemplate {
        compiled: compile_cached_thtml(&bound)?,
    })
}

#[pyfunction(signature = (template, globals = None, locals = None, registry = None))]
fn render_thtml_template(
    py: Python<'_>,
    template: &Bound<'_, PyAny>,
    globals: Option<&Bound<'_, PyDict>>,
    locals: Option<&Bound<'_, PyDict>>,
    registry: Option<&Bound<'_, PyAny>>,
) -> PyResult<String> {
    let bound = extract_template(py, template, "render_thtml_template")?;
    let (globals, locals) =
        normalize_scope_inputs(py, globals, locals, registry, "render_thtml_template")?;
    let compiled = compile_cached_thtml(&bound)?;
    let context = runtime_context_from_bound(py, &bound)?;
    render_thtml_document(
        py,
        compiled.document(),
        &context,
        globals.as_ref(),
        locals.as_ref(),
    )
}

#[pymodule]
fn tstring_html_bindings(py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__contract_version__", CONTRACT_VERSION)?;
    module.add("__contract_symbols__", CONTRACT_SYMBOLS)?;
    module.add("TemplateError", py.get_type::<TemplateError>())?;
    module.add("TemplateParseError", py.get_type::<TemplateParseError>())?;
    module.add(
        "TemplateSemanticError",
        py.get_type::<TemplateSemanticError>(),
    )?;
    module.add(
        "TemplateRuntimeError",
        py.get_type::<TemplateRuntimeError>(),
    )?;
    module.add_class::<Fragment>()?;
    module.add_class::<RawHtml>()?;
    module.add_class::<PyCompiledHtmlTemplate>()?;
    module.add_class::<PyCompiledThtmlTemplate>()?;
    module.add_function(wrap_pyfunction!(check_html_template, module)?)?;
    module.add_function(wrap_pyfunction!(format_html_template, module)?)?;
    module.add_function(wrap_pyfunction!(compile_html_template, module)?)?;
    module.add_function(wrap_pyfunction!(render_html_template, module)?)?;
    module.add_function(wrap_pyfunction!(render_html_fragment, module)?)?;
    module.add_function(wrap_pyfunction!(check_thtml_template, module)?)?;
    module.add_function(wrap_pyfunction!(format_thtml_template, module)?)?;
    module.add_function(wrap_pyfunction!(compile_thtml_template, module)?)?;
    module.add_function(wrap_pyfunction!(render_thtml_template, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ParseCache, render_thtml_node};
    use pyo3::Python;
    use tstring_html::{Node, RuntimeContext, RuntimeValue};

    #[test]
    fn parse_cache_reuses_entries() {
        let cache = ParseCache::new(2);
        let key = (
            "html".to_string(),
            "parse_validated".to_string(),
            vec!["<div>".to_string()],
        );
        let mut builds = 0;

        let first = cache
            .get_or_try_insert_with(&key, || {
                builds += 1;
                Ok::<_, ()>(1usize)
            })
            .expect("first insert should succeed");
        let second = cache
            .get_or_try_insert_with(&key, || {
                builds += 1;
                Ok::<_, ()>(2usize)
            })
            .expect("second lookup should succeed");

        assert_eq!(*first, 1);
        assert_eq!(*second, 1);
        assert_eq!(builds, 1);
    }

    #[test]
    fn parse_cache_evicts_lru_entry() {
        let cache = ParseCache::new(2);
        let key_a = (
            "html".to_string(),
            "parse_validated".to_string(),
            vec!["a".to_string()],
        );
        let key_b = (
            "html".to_string(),
            "parse_validated".to_string(),
            vec!["b".to_string()],
        );
        let key_c = (
            "html".to_string(),
            "parse_validated".to_string(),
            vec!["c".to_string()],
        );

        cache
            .get_or_try_insert_with(&key_a, || Ok::<_, ()>(1usize))
            .expect("insert a");
        cache
            .get_or_try_insert_with(&key_b, || Ok::<_, ()>(2usize))
            .expect("insert b");
        cache.get(&key_a).expect("touch a");
        cache
            .get_or_try_insert_with(&key_c, || Ok::<_, ()>(3usize))
            .expect("insert c");

        assert!(cache.get(&key_b).is_none());
        assert_eq!(*cache.get(&key_a).expect("a should remain"), 1);
        assert_eq!(*cache.get(&key_c).expect("c should remain"), 3);
    }

    #[test]
    fn parse_cache_does_not_store_failures() {
        let cache = ParseCache::new(2);
        let key = (
            "html".to_string(),
            "parse_validated".to_string(),
            vec!["a".to_string()],
        );
        let mut attempts = 0;

        let err = cache.get_or_try_insert_with(&key, || {
            attempts += 1;
            Err::<usize, _>("boom")
        });
        assert_eq!(err.expect_err("should fail"), "boom");
        assert!(cache.get(&key).is_none());

        let value = cache
            .get_or_try_insert_with(&key, || {
                attempts += 1;
                Ok::<_, &str>(7usize)
            })
            .expect("second attempt should succeed");
        assert_eq!(*value, 7);
        assert_eq!(attempts, 2);
    }

    #[test]
    fn render_thtml_node_treats_uppercase_title_as_escaped_text() {
        Python::attach(|py| {
            let node = Node::RawTextElement(tstring_html::RawTextElementNode {
                name: "TITLE".to_string(),
                attributes: Vec::new(),
                children: vec![Node::Interpolation(tstring_html::InterpolationNode {
                    interpolation_index: 0,
                    expression: "title".to_string(),
                    raw_source: Some("{title}".to_string()),
                    conversion: None,
                    format_spec: String::new(),
                    span: None,
                })],
                span: None,
            });
            let context = RuntimeContext {
                values: vec![RuntimeValue::RawHtml("<safe>".to_string())],
            };
            let mut out = String::new();
            render_thtml_node(py, &node, &context, None, None, &mut out)
                .expect("render uppercase title");
            assert_eq!(out, "<TITLE>&lt;safe&gt;</TITLE>");
        });
    }
}
