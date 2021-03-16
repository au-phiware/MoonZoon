pub use wasm_bindgen::{self, prelude::*, JsCast};
pub use blocks_macro::blocks;
pub use s_var_macro::s_var;
pub use update_macro::update;

pub mod element;
mod dom;
mod console;
mod hook;
mod l_var;
mod l_var_map;
mod s_var;
mod s_var_map;
mod runtime;

pub use element::*;
pub use dom::{Node, window, document}; 
pub use l_var::{LVar, CloneLVar};
pub use s_var::{SVar, CloneSVar, s_var};
pub use console::log;
pub use hook::{l_var, do_once};
pub use topo;
pub use topo::nested as render;
pub use topo::nested as cmp;
use runtime::ROOT_CMP;
pub use runtime::rerender;

#[macro_export]
macro_rules! with_dollar_sign {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
    }
}

const ELEMENT_ID: &str = "app";

#[macro_export]
macro_rules! start {
    () => {
        $crate::start(__blocks);
    };
    ($module_with_blocks:tt) => {
        $crate::start($module_with_blocks::__blocks);
    };
}

pub struct __Blocks {
    pub root: Option<Box<dyn Fn() -> Box<dyn Element>>>,
}

pub fn start(blocks: fn(__Blocks) -> __Blocks) {
    ROOT_CMP.with(move |root| {
        *root.borrow_mut() = blocks(__Blocks { root: None }).root;
    });

    // log!("start");
    console_error_panic_hook::set_once();

    rerender();
}

#[topo::nested]
fn root() {
    // log!("root");

    let node = l_var(|| Node {
        node_ws: web_sys::Node::from(document().get_element_by_id(ELEMENT_ID).expect("root element"))
    });

    let rcx = RenderContext { 
        index: 0,
        node 
    };

    ROOT_CMP.with(|app_root| {
        if let Some(app_root) = app_root.borrow_mut().as_ref() {
            app_root().render(rcx);
        }
    });
}
