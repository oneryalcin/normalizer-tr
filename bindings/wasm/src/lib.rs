use js_sys::{Array, Error, Object, Reflect};
use normalizer_tr::{
    AmbiguityPolicy, FallbackClass, FallbackDiagnostic, FallbackReason, FallbackStrategy, Hint,
    HintKind, Issue, IssueCategory, LimitKind, NormalizeError, NormalizeOptions, SegmentKind,
    SourceRange,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TYPES: &str = r#"
export type AmbiguityPolicy = "preserve" | "reject" | "fallback";
export type HintKind =
  | "cardinal" | "digits" | "date" | "time" | "ordinal"
  | "roman" | "range" | "telephone" | "electronic";

/** All byte offsets are UTF-8 offsets into the original text, not JS string indices. */
export interface Hint { startByte: number; endByte: number; kind: HintKind }
export interface NormalizeOptions { ambiguityPolicy?: AmbiguityPolicy; hints?: Hint[] }
export interface Segment { startByte: number; endByte: number; kind: string; text: string; ruleId: string }
export interface Issue { startByte: number; endByte: number; category: string; explanation: string }
export interface FallbackDiagnostic {
  startByte: number;
  endByte: number;
  attemptedClass: string;
  reason: string;
  originalCategory: string | null;
  strategy: string;
}
export interface NormalizeResult {
  normalizedText: string;
  locale: string;
  normalizerId: string;
  complete: boolean;
  segments: Segment[];
  issues: Issue[];
  fallbacks: FallbackDiagnostic[];
  fallbackUsed: boolean;
}
/** Thrown by `normalize`; `issues` is populated for code "unresolved". */
export interface NormalizationError extends Error {
  code: "invalid_input" | "invalid_hint" | "invalid_configuration" | "cancelled" | "internal" | "unresolved" | "limit_exceeded";
  limitKind: "input" | "hints" | "candidates" | "result" | null;
  issues: Issue[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "NormalizeOptions")]
    pub type JsOptions;
    #[wasm_bindgen(typescript_type = "NormalizeResult")]
    pub type JsResult;
}

fn set(object: &Object, key: &str, value: impl Into<JsValue>) {
    // Setting a data property on a fresh plain object cannot fail.
    let _ = Reflect::set(object, &JsValue::from_str(key), &value.into());
}

fn ranged(range: SourceRange) -> Object {
    let object = Object::new();
    set(&object, "startByte", range.start() as u32);
    set(&object, "endByte", range.end() as u32);
    object
}

fn issue_category(category: IssueCategory) -> &'static str {
    match category {
        IssueCategory::Ambiguous => "ambiguous",
        IssueCategory::InvalidExpression => "invalid_expression",
        IssueCategory::ProtectedIdentifier => "protected_identifier",
        IssueCategory::Unsupported => "unsupported",
        IssueCategory::UnknownAbbreviation => "unknown_abbreviation",
    }
}

fn issues(records: &[Issue]) -> Array {
    records
        .iter()
        .map(|issue| {
            let object = ranged(issue.range());
            set(&object, "category", issue_category(issue.category()));
            set(&object, "explanation", issue.explanation());
            object
        })
        .collect()
}

fn fallback(record: &FallbackDiagnostic) -> Object {
    let class = match record.attempted_class() {
        FallbackClass::Number => "number",
        FallbackClass::Date => "date",
        FallbackClass::Time => "time",
        FallbackClass::Percent => "percent",
        FallbackClass::Quantity => "quantity",
        FallbackClass::Abbreviation => "abbreviation",
        FallbackClass::Pronunciation => "pronunciation",
        FallbackClass::Identifier => "identifier",
        FallbackClass::Roman => "roman",
        FallbackClass::Electronic => "electronic",
        FallbackClass::Expression => "expression",
        FallbackClass::Symbol => "symbol",
    };
    let reason = match record.reason() {
        FallbackReason::MissingIntent => "missing_intent",
        FallbackReason::LeadingZeroes => "leading_zeroes",
        FallbackReason::InvalidForm => "invalid_form",
        FallbackReason::ProtectedIdentifier => "protected_identifier",
        FallbackReason::UnsupportedForm => "unsupported_form",
        FallbackReason::UnapprovedAbbreviation => "unapproved_abbreviation",
        FallbackReason::UnhandledSymbol => "unhandled_symbol",
    };
    let strategy = match record.strategy() {
        FallbackStrategy::PreferredNumber => "preferred_number",
        FallbackStrategy::PreferredDate => "preferred_date",
        FallbackStrategy::PreferredTime => "preferred_time",
        FallbackStrategy::SurfaceDate => "surface_date",
        FallbackStrategy::SurfaceTime => "surface_time",
        FallbackStrategy::Literal => "literal",
        FallbackStrategy::UnicodeCodePoint => "unicode_code_point",
    };
    let object = ranged(record.range());
    set(&object, "attemptedClass", class);
    set(&object, "reason", reason);
    set(
        &object,
        "originalCategory",
        record
            .original_category()
            .map_or(JsValue::NULL, |category| issue_category(category).into()),
    );
    set(&object, "strategy", strategy);
    object
}

fn segment_kind(kind: SegmentKind) -> &'static str {
    match kind {
        SegmentKind::Verbatim => "verbatim",
        SegmentKind::Unresolved => "unresolved",
        SegmentKind::Cardinal => "cardinal",
        SegmentKind::Ordinal => "ordinal",
        SegmentKind::Decimal => "decimal",
        SegmentKind::Digits => "digits",
        SegmentKind::Percent => "percent",
        SegmentKind::Money => "money",
        SegmentKind::Unit => "unit",
        SegmentKind::Date => "date",
        SegmentKind::Time => "time",
        SegmentKind::Abbreviation => "abbreviation",
        SegmentKind::Pronunciation => "pronunciation",
        SegmentKind::Range => "range",
        SegmentKind::Telephone => "telephone",
        SegmentKind::Iban => "iban",
        SegmentKind::Roman => "roman",
        SegmentKind::Electronic => "electronic",
        SegmentKind::Symbol => "symbol",
        SegmentKind::Fallback => "fallback",
    }
}

fn error(error: NormalizeError) -> JsValue {
    let (code, limit, diagnostics) = match &error {
        NormalizeError::InvalidInput => ("invalid_input", None, Array::new()),
        NormalizeError::InvalidHint => ("invalid_hint", None, Array::new()),
        NormalizeError::InvalidConfiguration => ("invalid_configuration", None, Array::new()),
        NormalizeError::Cancelled => ("cancelled", None, Array::new()),
        NormalizeError::Internal => ("internal", None, Array::new()),
        NormalizeError::Unresolved(records) => ("unresolved", None, issues(records)),
        NormalizeError::LimitExceeded(kind) => {
            let limit = match kind {
                LimitKind::Input => "input",
                LimitKind::Hints => "hints",
                LimitKind::Candidates => "candidates",
                LimitKind::Result => "result",
            };
            ("limit_exceeded", Some(limit), Array::new())
        }
    };
    let js = Error::new(&error.to_string());
    js.set_name("NormalizationError");
    set(&js, "code", code);
    set(&js, "limitKind", limit.map_or(JsValue::NULL, Into::into));
    set(&js, "issues", diagnostics);
    js.into()
}

fn type_error(message: &str) -> JsValue {
    js_sys::TypeError::new(message).into()
}

fn field(object: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    Reflect::get(object, &JsValue::from_str(key))
}

fn byte_offset(hint: &JsValue, key: &str) -> Result<usize, JsValue> {
    match field(hint, key)?.as_f64() {
        Some(value) if value >= 0.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX) => {
            Ok(value as usize)
        }
        _ => Err(type_error(&format!(
            "hint {key} must be a non-negative integer"
        ))),
    }
}

fn hint(value: JsValue) -> Result<Hint, JsValue> {
    if !value.is_object() {
        return Err(type_error("each hint must be an object"));
    }
    let kind = match field(&value, "kind")?.as_string().as_deref() {
        Some("cardinal") => HintKind::Cardinal,
        Some("digits") => HintKind::Digits,
        Some("date") => HintKind::Date,
        Some("time") => HintKind::Time,
        Some("ordinal") => HintKind::Ordinal,
        Some("roman") => HintKind::Roman,
        Some("range") => HintKind::Range,
        Some("telephone") => HintKind::Telephone,
        Some("electronic") => HintKind::Electronic,
        _ => return Err(type_error("unsupported hint kind")),
    };
    let range = SourceRange::new(
        byte_offset(&value, "startByte")?,
        byte_offset(&value, "endByte")?,
    );
    Ok(Hint::new(range, kind))
}

fn options(value: &JsValue) -> Result<NormalizeOptions, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(NormalizeOptions::default());
    }
    if !value.is_object() {
        return Err(type_error("options must be an object"));
    }
    let policy = field(value, "ambiguityPolicy")?;
    let ambiguity_policy = match policy.as_string().as_deref() {
        None if policy.is_undefined() => AmbiguityPolicy::Preserve,
        Some("preserve") => AmbiguityPolicy::Preserve,
        Some("reject") => AmbiguityPolicy::Reject,
        Some("fallback") => AmbiguityPolicy::Fallback,
        _ => {
            return Err(type_error(
                "ambiguityPolicy must be preserve, reject or fallback",
            ));
        }
    };
    let hints = field(value, "hints")?;
    let hints = if hints.is_undefined() {
        Vec::new()
    } else if Array::is_array(&hints) {
        // Every read goes through fallible `Reflect::get`: a getter or proxy that throws must
        // surface as an error, not unwind through the wasm frame. The count is bounded before
        // any element is read.
        let length = field(&hints, "length")?.as_f64().unwrap_or(0.0);
        if length > normalizer_tr::MAX_HINTS as f64 {
            return Err(error(NormalizeError::LimitExceeded(LimitKind::Hints)));
        }
        (0..length as u32)
            .map(|index| Reflect::get_u32(&hints, index).and_then(hint))
            .collect::<Result<_, _>>()?
    } else {
        return Err(type_error("hints must be an array of Hint"));
    };
    Ok(NormalizeOptions {
        ambiguity_policy,
        hints,
    })
}

/// Reusable Turkish normalizer backed by the Rust core.
#[wasm_bindgen]
pub struct Normalizer {
    inner: normalizer_tr::Normalizer,
}

#[wasm_bindgen]
impl Normalizer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Normalizer, JsValue> {
        let inner = normalizer_tr::Normalizer::new().map_err(error)?;
        Ok(Self { inner })
    }

    #[wasm_bindgen(getter, js_name = normalizerId)]
    pub fn normalizer_id(&self) -> String {
        self.inner.normalizer_id().to_owned()
    }

    /// Normalize `text`. Throws a `NormalizationError` on invalid input, hints or limits,
    /// and on unresolved spans under the "reject" policy.
    pub fn normalize(
        &self,
        #[wasm_bindgen(unchecked_param_type = "string")] text: JsValue,
        options: Option<JsOptions>,
    ) -> Result<JsResult, JsValue> {
        let text = text
            .as_string()
            .ok_or_else(|| type_error("text must be a string"))?;
        let options = options.map_or(JsValue::UNDEFINED, Into::into);
        let result = self
            .inner
            .normalize(&text, &self::options(&options)?)
            .map_err(error)?;
        let segments: Array = result
            .segments()
            .iter()
            .map(|segment| {
                let object = ranged(segment.range());
                set(&object, "kind", segment_kind(segment.kind()));
                set(&object, "text", segment.text());
                set(&object, "ruleId", segment.rule_id());
                object
            })
            .collect();
        let object = Object::new();
        set(&object, "normalizedText", result.normalized_text());
        set(&object, "locale", result.locale());
        set(&object, "normalizerId", result.normalizer_id());
        set(&object, "complete", result.complete());
        set(&object, "segments", segments);
        set(&object, "issues", issues(result.issues()));
        set(
            &object,
            "fallbacks",
            result.fallbacks().iter().map(fallback).collect::<Array>(),
        );
        set(&object, "fallbackUsed", result.fallback_used());
        Ok(JsValue::from(object).unchecked_into())
    }
}

#[wasm_bindgen(js_name = NORMALIZER_ID)]
pub fn normalizer_id() -> String {
    normalizer_tr::NORMALIZER_ID.to_owned()
}

#[wasm_bindgen(js_name = BUILD_VERSION)]
pub fn build_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}
