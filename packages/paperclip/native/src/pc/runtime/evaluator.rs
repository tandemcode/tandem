use super::super::ast;
use crate::base::runtime::{RuntimeError};
use crate::base::ast::{Location};
use super::virt;
use std::collections::HashSet;
use std::iter::FromIterator;
use super::graph::{DependencyGraph, DependencyContent};
use super::vfs::{VirtualFileSystem};
use crate::css::runtime::evaluator::{evaluate as evaluate_css};
use crate::js::runtime::evaluator::{evaluate as evaluate_js};
use crate::js::runtime::virt as js_virt;
use crate::js::ast as js_ast;
use crate::css::runtime::virt as css_virt;
use crate::base::utils::{get_document_style_scope, is_relative_path};
use crc::{crc32};


#[derive(Clone)]
pub struct Context<'a> {
  pub graph: &'a DependencyGraph,
  pub vfs: &'a VirtualFileSystem,
  pub uri: &'a String,  
  pub import_ids: HashSet<&'a String>,
  pub part_ids: HashSet<&'a String>,
  pub scope: String,
  pub data: &'a js_virt::JsValue,
  pub in_part: bool,
  pub id_seed: String,
  pub from_main: bool,
  pub id_count: i32
}

impl<'a> Context<'a> {
  pub fn get_next_id(&mut self) -> String {
    self.id_count += 1;
    format!("{}-{}", self.id_seed, self.id_count)
  }

  // pub fn run_with_new_data<FF, TRet>(&mut self, data: &'a js_virt::JsValue, run: FF) -> TRet 
  // where FF: Fn() -> TRet {

  // }
}

pub fn evaluate<'a>(node_expr: &ast::Node, uri: &String, graph: &'a DependencyGraph, vfs: &'a VirtualFileSystem, data: &js_virt::JsValue, part: Option<String>) -> Result<Option<virt::Node>, RuntimeError>  {
  let mut context = create_context(node_expr, uri, graph, vfs, data, None);
  let mut root_option = evaluate_instance_node(node_expr, &mut context, part)?;

  match root_option {
    Some(ref mut root) => {
      let style = evaluate_jumbo_style(node_expr, &mut context)?;
      root.prepend_child(style);
    },
    _ => { }
  }

  Ok(root_option)
}
pub fn evaluate_previews<'a>(previews: Vec<&ast::Node>, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError>  {
  let mut children  = vec![];
  for preview in previews {
    if let ast::Node::Element(element) = preview {
      let child_option = evaluate_component_instance(element, context.uri, context)?;
      if let Some(child) = child_option {
        children.push(child);
      }
    }
  }
  if children.len() == 1 {
    Ok(children.pop())
  } else {
    Ok(Some(virt::Node::Fragment(virt::Fragment {
      children
    })))
  }
}

pub fn evaluate_document_styles<'a>(node_expr: &ast::Node, uri: &String, vfs: &'a VirtualFileSystem) -> Result<css_virt::CSSSheet, RuntimeError>  {
  let mut sheet = css_virt::CSSSheet {
    rules: vec![] 
  };
  let children_option = ast::get_children(&node_expr);
  let scope = get_document_style_scope(uri);
  if let Some(children) = children_option {
    // style elements are only allowed in root, so no need to traverse
    for child in children {
      if let ast::Node::StyleElement(style_element) = &child {
        sheet.extend(evaluate_css(&style_element.sheet, uri, &scope, vfs)?);
      }
    }
  }

  Ok(sheet)
}

pub fn evaluate_jumbo_style<'a>(entry_expr: &ast::Node, context: &'a mut Context) -> Result<virt::Node, RuntimeError>  {

  let mut sheet = css_virt::CSSSheet {
    rules: vec![] 
  };
  let uri = context.uri;

  let deps =  context.graph.flatten(uri);

  for (dependency, dependent_option) in context.graph.flatten(uri) {

    // skip if self -- styles get evaluated after all imports. Note
    // that this is incorrect if style is declared before import, but whatever.
    if &dependency.uri == uri {
      continue;
    }
    let dep_sheet = match &dependency.content {
      DependencyContent::Node(node) => {
        evaluate_document_styles(node, &dependency.uri, context.vfs)?
      },
      DependencyContent::StyleSheet(sheet) => {
        let scope = if let Some(dependent) = dependent_option {
          get_document_style_scope(&dependent.uri)
        } else {
          get_document_style_scope(&dependency.uri)
        };
        
        evaluate_css(&sheet, &dependency.uri, &scope, context.vfs)?
      }
    };

    sheet.extend(dep_sheet);
  }

  // this element styles always get priority.
  sheet.extend(evaluate_document_styles(&entry_expr, &uri, context.vfs)?);

  
  Ok(virt::Node::StyleElement(virt::StyleElement {
    id: context.get_next_id(),
    sheet
  }))
}

pub fn evaluate_instance_node<'a>(node_expr: &ast::Node, context: &'a mut Context, part_option: Option<String>) -> Result<Option<virt::Node>, RuntimeError>  {
  evaluate_node(node_expr, true, context)
}

fn create_context<'a>(node_expr: &'a ast::Node, uri: &'a String, graph: &'a DependencyGraph, vfs: &'a VirtualFileSystem, data: &'a js_virt::JsValue,  parent_option: Option<&'a Context>) -> Context<'a> {

  let (from_main, curr_id_count) = if let Some(parent) = parent_option {
    (parent.from_main, parent.id_count)
  } else {
    (false, 0)
  };

  let scope = get_document_style_scope(uri);
  let id_seed = create_id_seed(uri, curr_id_count);

  Context {
    graph,
    uri,
    vfs,
    import_ids: HashSet::from_iter(ast::get_import_ids(node_expr)),
    part_ids: HashSet::from_iter(ast::get_part_ids(node_expr)),
    scope,
    data,
    in_part: false,
    from_main,
    id_seed,
    id_count: 0
  }
}

fn create_id_seed(uri: &String, curr_id_count: i32) -> String{
  format!("{:x}", crc32::checksum_ieee(format!("{}-{}", uri, curr_id_count).as_bytes())).to_string()
}

fn evaluate_node<'a>(node_expr: &ast::Node, is_root: bool, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  match &node_expr {
    ast::Node::Element(el) => {
      evaluate_element(&el, is_root, context)
    },
    ast::Node::StyleElement(el) => {
      evaluate_style_element(&el, context)
    },
    ast::Node::Text(text) => {
      Ok(Some(virt::Node::Text(virt::Text { 
        id: context.get_next_id(),
        value: text.value.to_string()
      })))
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

fn evaluate_element<'a>(element: &ast::Element, is_root: bool, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  match element.tag_name.as_str() {
    "import" => evaluate_import_element(element, context),
    "part" => evaluate_part_element(element, is_root, context),
    "self" => evaluate_self_element(element, context),
    "preview" => evaluate_preview_element(element, is_root, context),
    "script" | "property" | "logic" => Ok(None),
    _ => {
      if context.import_ids.contains(&ast::get_tag_name(&element)) {
        evaluate_imported_component(element, context)
      } else if context.part_ids.contains(&element.tag_name) {
        evaluate_part_instance_element(element, context)
      } else {
        evaluate_basic_element(element, context)
      }
    }
  }
}

fn evaluate_preview_element<'a>(element: &ast::Element, is_root: bool, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  Ok(None)
}

fn evaluate_slot<'a>(slot: &ast::Slot, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  let script = &slot.script;
  let mut js_value = evaluate_js(script, context)?;

  // if array of values, then treat as document fragment
  if let js_virt::JsValue::JsArray(ary) = &mut js_value {
    let mut children = vec![];
    for item in ary.values.drain(0..) {
      if let js_virt::JsValue::JsNode(child) = item {
        children.push(child);
      } else {
        children.push(virt::Node::Text(virt::Text {
          id: context.get_next_id(),
          value:item.to_string()
        }))
      }
    }

    return Ok(Some(virt::Node::Fragment(virt::Fragment {
      children
    })));
  } else if let js_virt::JsValue::JsNode(node)  = js_value {
    return Ok(Some(node));
  }

  Ok(Some(virt::Node::Text(virt::Text { 
    id: context.get_next_id(),
    value: js_value.to_string() 
  })))
}

fn evaluate_imported_component<'a>(element: &ast::Element, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  let self_dep  = &context.graph.dependencies.get(context.uri).unwrap();
  let dep_uri = &self_dep.dependencies.get(&ast::get_tag_name(element)).unwrap();
  evaluate_component_instance(element, dep_uri, context)
}


fn evaluate_self_element<'a>(element: &ast::Element, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  
  if context.from_main {
    return Err(RuntimeError { 
      uri: context.uri.to_string(), 
      message: "Can't call <self /> here since this causes an infinite loop!".to_string(), 
      location: element.open_tag_location.clone() 
    });
  }

  evaluate_component_instance(element, context.uri, context)
}

fn evaluate_part_instance_element<'a>(element: &ast::Element, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  let self_dep  = &context.graph.dependencies.get(context.uri).unwrap();

  if let DependencyContent::Node(root_node) = &self_dep.content {
    let part = ast::get_part_by_id(&element.tag_name, root_node).unwrap();
    let data = create_component_instance_data(element, context)?;
    let mut new_context = create_context(root_node, &self_dep.uri, context.graph, context.vfs, &data, Some(context));

    evaluate_element(part, true, &mut new_context)
  } else {

    // This should _never_ happen
    Err(RuntimeError::unknown(context.uri))
  }
}

fn create_component_instance_data<'a>(instance_element: &ast::Element, context: &'a mut Context) -> Result<js_virt::JsValue, RuntimeError> {
  let old_in_part = context.in_part;
  context.in_part = false;

  let mut data = js_virt::JsObject::new();

  for attr_expr in &instance_element.attributes {
    let attr = &attr_expr;
    let (name, value) = match attr {
      ast::Attribute::KeyValueAttribute(kv_attr) => {
        if kv_attr.value == None {
          (kv_attr.name.to_string(), js_virt::JsValue::JsBoolean(true))
        } else {
          let value = evaluate_attribute_value(&kv_attr.value.as_ref().unwrap(), context)?;
          (
            kv_attr.name.to_string(),
            value
          )
        }
      },
      ast::Attribute::ShorthandAttribute(sh_attr) => {
        let name = sh_attr.get_name().map_err(|message| {
          RuntimeError {
            uri: context.uri.to_string(),
            message: message.to_string(),
            location: Location { 
              start: 0,
              end: 0
            }
          }
        })?;

        (name.to_string(), evaluate_attribute_slot(&sh_attr.reference, context)?)
      }
    };

    data.values.insert(name, value);
  }

  
  let mut js_children = js_virt::JsArray::new();
  let children: Vec<js_virt::JsValue> = evaluate_children(&instance_element.children, context)?.into_iter().map(|child| {
    js_virt::JsValue::JsNode(child)
  }).collect();

  js_children.values.extend(children);

  data.values.insert("children".to_string(), js_virt::JsValue::JsArray(js_children));

  context.in_part = old_in_part;

  Ok(js_virt::JsValue::JsObject(data))
}

fn evaluate_component_instance<'a>(instance_element: &ast::Element, dep_uri: &String, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {

  let dep = &context.graph.dependencies.get(&dep_uri.to_string()).unwrap();
  let data = create_component_instance_data(instance_element, context)?;
  
  if let DependencyContent::Node(node) = &dep.content {

    let mut instance_parent_context = context.clone();
    instance_parent_context.from_main = false;


    let mut instance_context = create_context(&node, dep_uri, context.graph, context.vfs, &data, Some(&instance_parent_context));

    // TODO: if fragment, then wrap in span. If not, then copy these attributes to root element
    evaluate_instance_node(&node, &mut instance_context, None)
  } else {
    Err(RuntimeError::unknown(context.uri))
  }
}

fn evaluate_basic_element<'a>(element: &ast::Element, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {

  let mut attributes = vec![];

  let tag_name = ast::get_tag_name(element);

  for attr_expr in &element.attributes {
    let attr = &attr_expr;

    match attr {
      ast::Attribute::KeyValueAttribute(kv_attr) => {
        let (name, mut value_option) = if kv_attr.value == None {
          (kv_attr.name.to_string(), None)
        } else {
          (kv_attr.name.to_string(), Some(evaluate_attribute_value(&kv_attr.value.as_ref().unwrap(), context)?.to_string()))
        };

        if name == "src" {
          if let Some(value) = value_option {
            if is_relative_path(&value) {
              let full_path = context.vfs.resolve(context.uri, &value);
              value_option = Some(full_path);
            } else {
              value_option = None;
            }
          }
        }

        attributes.push(virt::Attribute {
          id: context.get_next_id(),
          name,
          value: value_option,
        });
      },
      ast::Attribute::ShorthandAttribute(sh_attr) => {
        let name = sh_attr.get_name().map_err(|message| {
          RuntimeError {
            uri: context.uri.to_string(),
            message: message.to_string(),
            location: Location { 
              start: 0,
              end: 0
            }
          }
        })?;
        let js_value = evaluate_attribute_slot(&sh_attr.reference, context)?;

        if js_value != js_virt::JsValue::JsUndefined() {
          attributes.push(virt::Attribute {
            id: context.get_next_id(),
            name: name.to_string(),
            value: Some(js_value.to_string()),
          });
        }
      }
    };

  }

  attributes.push(virt::Attribute {
    id: context.get_next_id(),
    name: format!("data-pc-{}", context.scope.to_string()).to_string(),
    value: None
  });

  let children = evaluate_children(&element.children, context)?;

  Ok(Some(virt::Node::Element(virt::Element {
    id: context.get_next_id(),
    source_uri: context.uri.to_string(),
    source_location: element.location.clone(),
    tag_name: tag_name,
    attributes,
    children
  })))
}

fn evaluate_import_element<'a>(_element: &ast::Element, _context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  Ok(None)
}

fn evaluate_part_element<'a>(element: &ast::Element, is_root: bool, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  if !is_root {
    return Ok(None)
  }

  let old_in_part = context.in_part;
  context.in_part = true;
  
  let result = evaluate_children_as_fragment(&element.children, context);
  context.in_part = old_in_part;
  result
}

fn evaluate_style_element<'a>(_element: &ast::StyleElement, _context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  Ok(None)
}
  

fn evaluate_children<'a>(children_expr: &Vec<ast::Node>, context: &'a mut Context) -> Result<Vec<virt::Node>, RuntimeError> {
  
  let mut children: Vec<virt::Node> = vec![];

  for child_expr in children_expr {
    match evaluate_node(child_expr, false, context)? {
      Some(c) => { children.push(c); },
      None => { }
    }
  }

  Ok(children)
}

fn evaluate_fragment<'a>(fragment: &ast::Fragment, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  evaluate_children_as_fragment(&fragment.children, context)
}

fn evaluate_children_as_fragment<'a>(children: &Vec<ast::Node>, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  let mut children = evaluate_children(&children, context)?;
  if children.len() == 1 {
    return Ok(children.pop());
  }
  Ok(Some(virt::Node::Fragment(virt::Fragment {
    children
  })))
}

fn evaluate_block<'a>(block: &ast::Block, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  match block {
    ast::Block::Conditional(conditional_block) => {
      evaluate_conditional_block(conditional_block, context)
    },
    ast::Block::Each(each_block) => {
      evaluate_each_block(each_block, context)
    }
  }
}

fn evaluate_conditional_block<'a>(block: &ast::ConditionalBlock, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  match block {
    ast::ConditionalBlock::PassFailBlock(pass_fail) => {
      evaluate_pass_fail_block(pass_fail, context)
    },
    ast::ConditionalBlock::FinalBlock(block) => {
      if let Some(node) = &block.body {
        evaluate_node(node, false, context)
      } else {
        Ok(None)
      }
    }
  }
}

fn evaluate_pass_fail_block<'a>(block: &ast::PassFailBlock, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {
  let condition = evaluate_js(&block.condition, context)?;
  if condition.truthy() {
    if let Some(node) = &block.body {
      evaluate_node(node, false, context)
    } else if let Some(fail) = &block.fail {
      evaluate_conditional_block(fail, context)
    } else {
      Ok(None)
    }
  } else if let Some(fail) = &block.fail {
    evaluate_conditional_block(fail, context)
  } else {
    Ok(None)
  }
}

fn evaluate_each_block<'a>(block: &ast::EachBlock, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {

  if block.body == None {
    return Ok(None)
  }

  let body = block.body.as_ref().unwrap();

  let mut children: Vec<virt::Node> = vec![];
  let source = evaluate_js(&block.source, context)?;

  if let js_virt::JsValue::JsArray(items) = source {
    for (index, item) in items.values.iter().enumerate() {
      let child_option = evaluate_each_block_child(&body, item, index, &block.value_name, &block.key_name, context)?;
      if let Some(child) = child_option {
        children.push(child);
      }
    }

  } else {

  }

  Ok(Some(virt::Node::Fragment(virt::Fragment {
    children,
  })))
}

fn evaluate_each_block_child<'a>(body: &ast::Node, item: &js_virt::JsValue, index: usize, item_name: &String, key_name: &Option<String>, context: &'a mut Context) -> Result<Option<virt::Node>, RuntimeError> {

  let mut data = context.data.clone();
  match data {
    js_virt::JsValue::JsObject(ref mut data) => {
      data.values.insert(item_name.to_string(), item.clone());
      if let Some(key) = key_name {
        let key_value = js_virt::JsValue::JsNumber(index as f64);
        data.values.insert(key.to_string(), key_value);
      }
    },
    _ => { }
  }  
  context.id_count += 1;
  // let mut child_context = create_context(contex, context.uri, context.graph, context.vfs, &data, Some(context));
  let mut child_context = context.clone();
  child_context.id_count = 0;
  child_context.id_seed = create_id_seed(context.uri, context.id_count);
  child_context.data = &data;

  evaluate_node(body, false, &mut child_context)
}

fn evaluate_attribute_value<'a>(value: &ast::AttributeValue, context: &mut Context) -> Result<js_virt::JsValue, RuntimeError> {
  match value {
    ast::AttributeValue::String(st) => {
      Ok(js_virt::JsValue::JsString(st.value.clone()))
    }
    ast::AttributeValue::Slot(script) => {
      evaluate_attribute_slot(script, context)
    }
  }
}

fn evaluate_attribute_slot<'a>(script: &js_ast::Statement, context: &'a mut Context) -> Result<js_virt::JsValue, RuntimeError> {
  evaluate_js(script, context)
}

#[cfg(test)]
mod tests {
  use super::*;
  use super::super::super::parser::*;

  #[test]
  fn can_evaluate_a_style() {
    let case = "<style>div { color: red; }</style><div></div>";
    let ast = parse(case).unwrap();
    let graph = DependencyGraph::new();
    let vfs = VirtualFileSystem::new(Box::new(|_| "".to_string()), Box::new(|_| true), Box::new(|_,_| "".to_string()));
    let _node = evaluate(&ast, &"something".to_string(), &graph, &vfs, &js_virt::JsValue::JsObject(js_virt::JsObject::new()), None).unwrap().unwrap();
  }

  #[test]
  fn can_evaluate_a_simple_each_block() {
    let code = "{#each items as item}{item}{/}";
    let ast = parse(code).unwrap();
    let graph = DependencyGraph::new();
    let vfs = VirtualFileSystem::new(Box::new(|_| "".to_string()), Box::new(|_| true), Box::new(|_,_| "".to_string()));

    let mut object = js_virt::JsObject::new();
    let mut items = js_virt::JsArray::new();
    items.values.push(js_virt::JsValue::JsString("a".to_string()));
    items.values.push(js_virt::JsValue::JsString("b".to_string()));
    items.values.push(js_virt::JsValue::JsString("c".to_string()));
    object.values.insert("items".to_string(), js_virt::JsValue::JsArray(items));
    let data = js_virt::JsValue::JsObject(object);
    let _node = evaluate(&ast, &"some-file.pc".to_string(), &graph, &vfs, &data, None).unwrap().unwrap();
  }

  #[test]
  fn can_smoke_evaluate_various_elements() {

    let cases = [
      "{#each [1, 2, 3] as item}{item}{/}",
      "{#if true}do something{/}",
      "{#if false}do something{/else}something else{/}",
      "
      <span>
        {#each [0, false, 1, true] as item}
          {#if item}
            pass: {item}
          {/else}
            fail: {item}
          {/}
        {/}
      </span>
      {#each [1, 2, 3] as item}
  okay
{/}

    {\"a\"}
        
      "
    ];
    
    for code in cases.iter() {
      let ast = parse(code).unwrap();
      let graph = DependencyGraph::new();
      let vfs = VirtualFileSystem::new(Box::new(|_| "".to_string()), Box::new(|_| true), Box::new(|_,_| "".to_string()));

      let data = js_virt::JsValue::JsObject(js_virt::JsObject::new());
      let _node = evaluate(&ast, &"some-file.pc".to_string(), &graph, &vfs, &data, None).unwrap().unwrap();
      println!("{:?}", _node);
    }
  }
}
