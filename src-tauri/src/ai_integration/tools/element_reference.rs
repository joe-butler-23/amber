use crate::elements::value_objects::element_id::ElementId;

/// Rendered by the frontend as a link to the element; keep in sync with `elementReference.ts`.
pub const ELEMENT_REFERENCE_TAG: &str = "element-ref";

/// Markdown linking to an element, which the frontend labels with its current name.
pub fn element_reference(element_id: ElementId) -> String {
    let value = serde_json::to_value(element_id).expect("an ElementId always serializes");
    let element_type = value["type"].as_str().unwrap_or_default();
    format!(
        r#"<{ELEMENT_REFERENCE_TAG} data-type="{element_type}" data-id="{}"></{ELEMENT_REFERENCE_TAG}>"#,
        element_id.id().hyphenated()
    )
}
