//! **Every error says which kind of failure it is** (D193). The view has to tell "the player cannot do that" from "the engine is
//! wrong" - one is a message, the other is a bug - and with 59 variants in one enum the only way to know is to match all of them.
//!
//! Written before `SimError::class` existed. The guarantee that makes it durable is structural rather than a test: the classification
//! is a **match with no wildcard arm**, so a new variant does not compile until it is classified.

use ruage_sim::{EntityId, ErrorClass, SimError};

#[test]
fn a_legal_order_that_cannot_be_obeyed_is_a_refusal() {
    assert_eq!(
        SimError::NoPath {
            from: (3, 3),
            to: (9, 9)
        }
        .class(),
        ErrorClass::Refused
    );
    assert_eq!(
        SimError::NotEnough {
            resource: "wood".to_owned(),
            need: 30,
            have: 10
        }
        .class(),
        ErrorClass::Refused
    );
    assert_eq!(
        SimError::SiteNotClear { tile: (5, 5) }.class(),
        ErrorClass::Refused
    );
    assert_eq!(SimError::MatchOver.class(), ErrorClass::Refused);
}

#[test]
fn a_command_naming_something_that_is_not_there_is_a_bad_request() {
    assert_eq!(
        SimError::MissingEntity(EntityId(7)).class(),
        ErrorClass::BadRequest
    );
    assert_eq!(
        SimError::NotYours {
            id: EntityId(4),
            player: 2
        }
        .class(),
        ErrorClass::BadRequest
    );
}

#[test]
fn arithmetic_the_engine_cannot_answer_for_is_internal() {
    assert_eq!(
        SimError::Overflow(EntityId(3)).class(),
        ErrorClass::Internal
    );
}
