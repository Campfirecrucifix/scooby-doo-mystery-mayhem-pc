//! Engine API slots implemented by the port itself (they only touch the logic runtime / module memory).
//! Everything else is forwarded to the platform [`crate::engine::Host`] by the generated glue.
#![allow(non_snake_case, unused_variables)]
pub mod apicons;
pub mod apiextra;
pub mod apigame;
pub mod apigameobj;
pub mod apimath;
pub mod apiloadsave;
pub mod apireglib;
pub mod rwmath;
