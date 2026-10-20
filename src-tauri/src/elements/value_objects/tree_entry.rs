use fractional_index::FractionalIndex;

use crate::elements::value_objects::element_id::ElementId;

pub struct TreeEntry {
    pub element_id: ElementId,
    pub name: String,
    pub position: FractionalIndex,
    pub parent: Option<ElementId>,
}
