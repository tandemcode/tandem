use super::super::ast;
use super::virt;
use std::collections::HashSet;
use std::iter::FromIterator;
use super::graph::{DependencyGraph};
use crate::css::runtime::evaulator::{evaluate as evaluate_css};
use crate::js::runtime::evaluator::{evaluate as evaluate_js};
use crate::js::runtime::virt as js_virt;
use crate::js::ast as js_ast;
use crate::css::runtime::virt as css_virt;
use crc::{crc32};

#[derive(Debug)]
pub struct Context<'a> {
  graph: &'a DependencyGraph,
  file_path: &'a String,
  import_ids: HashSet<&'a String>,
  scope: String,
  data: &'a js_virt::JsValue,
}

pub fn evaluate<'a>(node_expr: &ast::Node, file_path: &String, graph: &'a DependencyGraph, data: &js_virt::JsValue) -> Result<Option<virt::Node>, &'static str>  {
  let context = create_context(node_expr, file_path, graph, data);
  
  let mut root_result = evaluate_node(node_expr, &context);

  // need to insert all styles into the root for efficiency
  match root_result {
    Ok(ref mut root_option) => {
      match root_option {
        Some(ref mut root) => {
          let style = evaluate_jumbo_style(node_expr, file_path, graph)?;
          root.prepend_child(style);
        },
        _ => { }
      }
    }
    _ => { }
  };

  root_result
}


pub fn evaluate_jumbo_style<'a>(_entry_expr: &ast::Node,  file_path: &String, graph: &'a DependencyGraph) -> Result<virt::Node, &'static str>  {

  let mut sheet = css_virt::CSSSheet {
    rules: vec![] 
  };

  for dep in graph.flatten(file_path) {
    let children_option = ast::get_children(&dep.expression);
    let scope = get_component_scope(&dep.file_path);
    if let Some(children) = children_option {

      // style elements are only allowed in root, so no need to traverse
      for child in children {
        if let ast::Node::StyleElement(style_element) = &child {
          sheet.extend(evaluate_css(&style_element.sheet, &scope)?);
        }
      }
    }
  }
  
  Ok(virt::Node::StyleElement(virt::StyleElement {
    sheet
  }))
}

pub fn evaluate_component<'a>(node_expr: &ast::Node, file_path: &String, graph: &'a DependencyGraph, data: &js_virt::JsValue) -> Result<Option<virt::Node>, &'static str>  {
  evaluate_node(node_expr, &create_context(node_expr, file_path, graph, data))
}

fn create_context<'a>(node_expr: &'a ast::Node, file_path: &'a String, graph: &'a DependencyGraph, data: &'a js_virt::JsValue) -> Context<'a> {
  Context {
    graph,
    file_path,
    import_ids: HashSet::from_iter(ast::get_import_ids(node_expr)),
    scope: get_component_scope(file_path),
    data
  }
}

fn get_component_scope<'a>(file_path: &String) -> String {
  format!("{:x}", crc32::checksum_ieee(file_path.as_bytes())).to_string()
}

fn evaluate_node<'a>(node_expr: &ast::Node, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  match &node_expr {
    ast::Node::Element(el) => {
      evaluate_element(&el, context)
    },
    ast::Node::StyleElement(el) => {
      evaluate_style_element(&el, context)
    },
    ast::Node::Text(text) => {
      Ok(Some(virt::Node::Text(virt::Text { value: text.value.to_string() })))
    },
    ast::Node::Slot(slot) => {
      evaluate_slot(&slot, context)
    },
    ast::Node::Fragment(el) => {
      evaluate_fragment(&el, context)
    },
    ast::Node::Block(block) => {
      evaluate_block(&block, context)
    },
    ast::Node::Comment(_el) => {
      Ok(None)
    }
  }
}

fn evaluate_element<'a>(element: &ast::Element, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  match element.tag_name.as_str() {
    "import" => evaluate_import_element(element, context),
    _ => {
      if context.import_ids.contains(&element.tag_name) {
        evaluate_imported_component(element, context)
      } else {
        evaluate_basic_element(element, context)
      }
    }
  }
}


fn evaluate_slot<'a>(slot: &js_ast::Statement, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  let mut js_value = evaluate_js(slot, &context.data)?;

  // if array of values, then treat as document fragment
  if let js_virt::JsValue::JsArray(ary) = &mut js_value {
    let mut children = vec![];
    for item in ary.values.drain(0..) {
      if let js_virt::JsValue::JsNode(child) = item {
        children.push(child);
      } else {
        children.push(virt::Node::Text(virt::Text {
          value:item.to_string()
        }))
      }
    }

    return Ok(Some(virt::Node::Fragment(virt::Fragment {
      children
    })));
  }

  Ok(Some(virt::Node::Text(virt::Text { value: js_value.to_string() })))
}

fn evaluate_imported_component<'a>(element: &ast::Element, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {

  let self_dep  = &context.graph.dependencies.get(context.file_path).unwrap();
  let dep_file_path = &self_dep.dependencies.get(&element.tag_name).unwrap();
  let dep = &context.graph.dependencies.get(&dep_file_path.to_string()).unwrap();
  let mut data = js_virt::JsObject::new();

  for attr_expr in &element.attributes {
    let attr = &attr_expr;
    let (name, value) = match attr {
      ast::Attribute::KeyValueAttribute(kv_attr) => {
        if kv_attr.value == None {
          (kv_attr.name.to_string(), js_virt::JsValue::JsBoolean(true))
        } else {
          let value = evaluate_attribute_value(&kv_attr.value.as_ref().unwrap(), context)?;
          (
            kv_attr.name.to_string(),
            js_virt::JsValue::JsString(value.unwrap().to_string())
          )
        }
      },
      ast::Attribute::ShorthandAttribute(sh_attr) => {
        let name = sh_attr.get_name()?;

        (name.to_string(), evaluate_attribute_slot(&sh_attr.reference, context)?)
      }
    };

    data.values.insert(name, value);
  }

  
  let mut js_children = js_virt::JsArray::new();
  let children: Vec<js_virt::JsValue> = evaluate_children(&element.children, &context)?.into_iter().map(|child| {
    js_virt::JsValue::JsNode(child)
  }).collect();

  js_children.values.extend(children);

  data.values.insert("children".to_string(), js_virt::JsValue::JsArray(js_children));

  // TODO: if fragment, then wrap in span. If not, then copy these attributes to root element
  evaluate_component(&dep.expression, dep_file_path, &context.graph, &js_virt::JsValue::JsObject(data))
}

fn evaluate_basic_element<'a>(element: &ast::Element, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  
  let mut attributes = vec![];
  

  let tag_name = element.tag_name.clone();

  for attr_expr in &element.attributes {
    let attr = &attr_expr;

    let (name, value) = match attr {
      ast::Attribute::KeyValueAttribute(kv_attr) => {
        if kv_attr.value == None {
          (kv_attr.name.to_string(), None)
        } else {
          (kv_attr.name.to_string(), evaluate_attribute_value(&kv_attr.value.as_ref().unwrap(), context)?)
        }
      },
      ast::Attribute::ShorthandAttribute(sh_attr) => {
        let name = sh_attr.get_name()?;
        let value = evaluate_attribute_slot(&sh_attr.reference, context)?;
        (name.to_string(), Some(value.to_string()))
      }
    };


    attributes.push(virt::Attribute {
      name, 
      value,
    });
  }

  attributes.push(virt::Attribute {
    name: format!("data-pc-{}", context.scope.to_string()).to_string(),
    value: None
  });

  let children = evaluate_children(&element.children, context)?;

  Ok(Some(virt::Node::Element(virt::Element {
    tag_name: tag_name,
    attributes,
    children
  })))
}

fn evaluate_import_element<'a>(_element: &ast::Element, _context: &'a Context) -> Result<Option<virt::Node>, &'static str> {

  // TODO - import into context
  Ok(None)
}

fn evaluate_style_element<'a>(_element: &ast::StyleElement, _context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  Ok(None)
  // Ok(Some(virt::Node::StyleElement(virt::StyleElement {
  //   sheet: evaluate_css(&element.sheet, &context.scope)?
  // })))
}
  

fn evaluate_children<'a>(children_expr: &Vec<ast::Node>, context: &'a Context) -> Result<Vec<virt::Node>, &'static str> {
  
  let mut children: Vec<virt::Node> = vec![];

  for child_expr in children_expr {
    match evaluate_node(child_expr, context)? {
      Some(c) => { children.push(c); },
      None => { }
    }
  }

  Ok(children)
}

fn evaluate_fragment<'a>(fragment: &ast::Fragment, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  let mut children = evaluate_children(&fragment.children, context)?;
  if children.len() == 1 {
    return Ok(children.pop());
  }
  Ok(Some(virt::Node::Fragment(virt::Fragment {
    children
  })))
}

fn evaluate_block<'a>(block: &ast::Block, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  match block {
    ast::Block::Conditional(conditional) => {
      evaluate_conditional(conditional, context)
    }
  }
}

fn evaluate_conditional<'a>(block: &ast::ConditionalBlock, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  match block {
    ast::ConditionalBlock::PassFailBlock(pass_fail) => {
      evaluate_pass_fail_block(pass_fail, context)
    },
    ast::ConditionalBlock::FinalBlock(block) => {
      if let Some(node) = &block.node {
        evaluate_node(node, context)
      } else {
        Ok(None)
      }
    }
  }
}

fn evaluate_pass_fail_block<'a>(block: &ast::PassFailBlock, context: &'a Context) -> Result<Option<virt::Node>, &'static str> {
  let condition = evaluate_js(&block.condition, context.data)?;
  if condition.truthy() {
    if let Some(node) = &block.node {
      evaluate_node(node, context)
    } else if let Some(fail) = &block.fail {
      evaluate_conditional(fail, context)
    } else {
      Ok(None)
    }
  } else if let Some(fail) = &block.fail {
    evaluate_conditional(fail, context)
  } else {
    Ok(None)
  }
}

fn evaluate_attribute_value<'a>(value: &ast::AttributeValue, context: &'a Context) -> Result<Option<String>, &'static str> {
  match value {
    ast::AttributeValue::String(st) => {
      Ok(Some(st.value.clone()))
    }
    ast::AttributeValue::Slot(script) => {
      Ok(Some(evaluate_attribute_slot(script, context)?.to_string()))
    }
  }
}

// fn evaluate_component_property<'a>(value: &ast::AttributeValue, context: &'a Context) -> Result<js_virt::JsValue, &'static str> {
//   match value {
//     ast::AttributeValue::String(st) => {
//       Ok(js_virt::JsValue::JsString(st.value.clone()))
//     }
//     ast::AttributeValue::Slot(script) => {
//       Ok(evaluate_attribute_slot(script, context)?)
//     }
//   }
// }

fn evaluate_attribute_slot<'a>(script: &js_ast::Statement, context: &'a Context) -> Result<js_virt::JsValue, &'static str> {
  evaluate_js(script, &context.data)
}

#[cfg(test)]
mod tests {
  use super::*;
  use super::super::graph::*;
  use super::super::super::parser::*;

  #[test]
  fn can_evaluate_a_style() {
    let case = "<style>div { color: red; }</style><div></div>";
    let ast = parse(case).unwrap();
    let graph = DependencyGraph::new();
    let _node = evaluate(&ast, &"something".to_string(), &graph, &js_virt::JsValue::JsObject(js_virt::JsObject::new())).unwrap().unwrap();
  }
}