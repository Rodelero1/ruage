//! What a price is made of, shared by everything that costs anything (D17).
//!
//! A unit costs resources and a tech costs resources, so they use one type. A cost names its
//! resource in one of two namespaces:
//!
//! - `ReferenceId(u8)` - the reference content pack's numbering, which is what the generator emits
//!   (`[[1, 25]]` is 25 wood);
//! - `Name(String)` - a resource name the pack defines, which is what a pack of our own writes.
//!
//! The pack is the only thing that turns either into a stockpile column, so nothing above the pack
//! layer learns what the number or the word meant.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostResource {
    ReferenceId(u8),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cost {
    pub resource: CostResource,
    pub amount: u16,
}

/// `[1, 25]` is the reference pack's id; `{ resource = "timber", amount = 25 }` is a name. Untagged
/// so the generated tables (arrays, hundreds of KB) are untouched by the form our own content needs.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RawCost {
    Reference([i64; 2]),
    Named { resource: String, amount: u16 },
}

/// Convert one wire entry, naming the file and key in every refusal.
pub(crate) fn convert(raw: &RawCost, key: impl Fn() -> String) -> Result<Cost, crate::ConfigError> {
    match raw {
        RawCost::Reference([id, amount]) => {
            let resource = u8::try_from(*id).map_err(|_| crate::ConfigError::Invalid {
                key: key(),
                detail: format!("{id} is not a resource id"),
            })?;
            let amount = u16::try_from(*amount).map_err(|_| crate::ConfigError::Invalid {
                key: key(),
                detail: format!("{amount} does not fit a u16"),
            })?;
            Ok(Cost {
                resource: CostResource::ReferenceId(resource),
                amount,
            })
        }
        // Our own content: a name the pack defines, so an original pack never borrows the reference
        // pack's numbering (D17).
        RawCost::Named { resource, amount } => {
            let name = resource.trim();
            if name.is_empty() {
                return Err(crate::ConfigError::Invalid {
                    key: key(),
                    detail: "a named cost needs a resource name".to_owned(),
                });
            }
            Ok(Cost {
                resource: CostResource::Name(name.to_owned()),
                amount: *amount,
            })
        }
    }
}
