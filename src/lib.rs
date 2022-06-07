#![allow(clippy::all)]
#![allow(unused_parens)]
//! # TAP query
//!
//! `TAP query` is a library that let you easily find, search and filter data from a TAP tree. 
#[macro_use] extern crate lalrpop_util;

#[deny(clippy::all)]
pub mod filter;  
pub mod attribute;
pub mod timeline;
pub mod data;
lalrpop_mod!(pub parser);
