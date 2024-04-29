use std::collections::{HashMap, HashSet};

use crate::{
    css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value},
    dom::{ElementData, Node, NodeType},
};

type PropertyMap = HashMap<String, Value>;

enum Display {
    Inline,
    Block,
    None,
}

#[derive(Debug)]
pub struct StyledNode<'a> {
    node: &'a Node,
    specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

impl StyledNode<'_> {
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).map(|v| v.clone())
    }
    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match &*s {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }
}

type MatchedRule<'a> = (Specificity, &'a Rule);

pub fn style_tree<'a>(
    root: &'a Node,
    stylesheet: &'a Stylesheet,
    parent_style: Option<&PropertyMap>,
) -> StyledNode<'a> {
    let current_style = match &root.node_type {
        NodeType::Element(ref elem) => specified_values(elem, stylesheet, parent_style),
        NodeType::Text(_) => parent_style.cloned().unwrap_or_default(),
    };

    let children_styles = root
        .children
        .iter()
        .map(|child| style_tree(child, stylesheet, Some(&current_style)))
        .collect();

    StyledNode {
        node: root,
        specified_values: current_style,
        children: children_styles,
    }
}

fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match *selector {
        Selector::Simple(ref simple_selector) => matches_simple_selector(elem, simple_selector),
    }
}

fn matches_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
        return false;
    }
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    let elem_classes = elem.classes();
    if selector
        .class
        .iter()
        .any(|class| !elem_classes.contains(&**class))
    {
        return false;
    }

    true
}

fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    rule.selectors
        .iter()
        .find(|selector| matches(elem, *selector))
        .map(|selector| (selector.specificity(), rule))
}

fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule))
        .collect()
}

fn specified_values(
    elem: &ElementData,
    stylesheet: &Stylesheet,
    parent_style: Option<&PropertyMap>,
) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }

    let inheritable_props = inheritable_properties();
    if let Some(parent_style) = parent_style {
        for &prop in inheritable_props.iter() {
            if !values.contains_key(prop) {
                if let Some(value) = parent_style.get(prop) {
                    values.insert(prop.to_string(), value.clone());
                }
            }
        }
    }

    values
}

fn inheritable_properties() -> HashSet<&'static str> {
    let mut props = HashSet::new();
    props.insert("color");
    props.insert("font-family");
    props
}
