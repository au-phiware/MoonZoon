use crate::*;

#[derive(Default)]
pub struct Spacing<'a> {
    static_css_props: StaticCSSProps<'a>,
}

impl<'a> Spacing<'a> {
    pub fn new(spacing: u32) -> Self {
        let mut this = Self::default();
        this.static_css_props.insert("gap", px(spacing));
        this
    }
}

impl<'a> Style<'a> for Spacing<'a> {
    fn apply_to_raw_el<E: RawEl>(
        self,
        mut raw_el: E,
        style_group: Option<StyleGroup<'a>>,
    ) -> (E, Option<StyleGroup<'a>>) {
        if let Some(mut style_group) = style_group {
            for (name, css_prop_value) in self.static_css_props {
                style_group = if css_prop_value.important {
                    style_group.style(name, css_prop_value.value)
                } else {
                    style_group.style_important(name, css_prop_value.value)
                };
            }
            return (raw_el, Some(style_group));
        }
        for (name, css_prop_value) in self.static_css_props {
            raw_el = if css_prop_value.important {
                raw_el.style_important(name, &css_prop_value.value)
            } else {
                raw_el.style(name, &css_prop_value.value)
            };
        }
        (raw_el, None)
    }
}
