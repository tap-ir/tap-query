//! Method and Struct use to filter [Node] [Attribute].

use tap::node::Node;
use tap::attribute::Attribute;
use tap::value::{ValueTypeId, Value};
use tap::tree::{Tree, TreeNodeId};

use regex::Regex;
use wildmatch::WildMatch;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::clangd::ClangdMatcher;
use rayon::prelude::*;
use anyhow::Result;

/**
 *  Different matching methods used by [MatcherMethod].
 */ 
#[derive(Debug)]
pub enum MatchMethod
{
  /// Compare full string
  Fixed,
  /// Compare using regexp
  Regex,
  /// Compare using wildcard
  Wildcard,
  /// Compare using fuzzy matching
  Fuzzy,
}

/**
 *  Generate matcher for different available [MatchMethod].
 */
pub enum MatcherMethod
{
  Fixed,
  Regex(Regex),
  Wildcard(WildMatch),
  Fuzzy(ClangdMatcher),
}

impl MatcherMethod
{
  /// Instantiate a new Matcher using `method_type` compiled with `query`.
  pub fn new(method_type : &MatchMethod, query : &str) -> Result<MatcherMethod>
  {
    match method_type 
    {
      MatchMethod::Fixed => Ok(MatcherMethod::Fixed),
      MatchMethod::Regex => Ok(MatcherMethod::Regex(Regex::new(query)?)),
      MatchMethod::Wildcard => Ok(MatcherMethod::Wildcard(WildMatch::new(query))), 
      MatchMethod::Fuzzy => Ok(MatcherMethod::Fuzzy(ClangdMatcher::default())),
    }
  }

  /// Check if string `query` match `value` using self [`MatcherMethod`].
  pub fn is_match(&self, query : &str, value: &str) -> bool
  {
    match &self
    {
      MatcherMethod::Fixed => value == query,
      MatcherMethod::Regex(matcher) => matcher.is_match(value),
      MatcherMethod::Wildcard(matcher) => matcher.matches(value),
      MatcherMethod::Fuzzy(matcher) => matcher.fuzzy_match(value, query).is_some()
    }
  }
}

/**
 *  Select on which nodes variable attribute is matched.
 */
#[derive(Debug)]
pub enum QueryType
{
  /// Match on a node attribute name.
  AttributeName,
  /// Match on node name.
  Name,
}

/**
 *  Multithreaded function that iterate on `nodes` and return if `query_value` matched [Node] [Attribute] using `match_method_type` [MatcherMethod].
 */
pub fn match_query(tree : &Tree, nodes : &Vec<TreeNodeId>, query_type : QueryType, match_method_type : MatchMethod, query_value : &str) -> Result<Vec<TreeNodeId>>
{
  //We reuse the same matcher in every thread (there should be all multithreadable)
  let matcher = MatcherMethod::new(&match_method_type, query_value)?;

  Ok(nodes.par_iter().filter_map(|node_id|
  {
     if let Some(node) = tree.get_node_from_id(*node_id)
     {
       let is_match = match query_type //match query type for each node, can do it one time
       {
         //Compare node name to query value
         QueryType::Name => matcher.is_match(query_value, &node.name()),
         QueryType::AttributeName => match_attributes_dotted_name(&node, query_value, &matcher),
       }; 
       if is_match 
       {
         return Some(*node_id)
       }
     }
     None
  }).collect())
}

fn match_attributes_dotted_name(node : &Node, query_value : &str, matcher: &MatcherMethod) -> bool
{
  for attribute in node.value().attributes().iter()
  {
    if match_attribute_dotted_name("".into(), &attribute, &query_value, &matcher) 
    {
      return true
    }
  }
  return false
}

fn match_attribute_dotted_name(dotted_attrib: String, attribute : &Attribute, query_value : &str, matcher: &MatcherMethod) -> bool 
{
  if attribute.type_id() == ValueTypeId::Attributes
  {
      for current_attribute in attribute.value().as_attributes().attributes().iter()
      {
        let dotted_attrib = match dotted_attrib.len() 
        {
          0 => attribute.name().to_string(),
          _ => dotted_attrib.to_string() + "." + attribute.name(),
        };
        if match_attribute_dotted_name(dotted_attrib, &current_attribute, &query_value, &matcher)
        {
          return true
        }
      }
  }
  else if attribute.type_id() == ValueTypeId::ReflectStruct 
  {
      let attributes : Vec<Attribute> = attribute.value().as_reflect_struct().attributes();
      for current_attribute in attributes.iter() 
      {
        let dotted_attrib = match dotted_attrib.len() 
        {
          0 => attribute.name().to_string(),
          _ => dotted_attrib.to_string() + "." + attribute.name(),
        };
        if match_attribute_dotted_name(dotted_attrib, &current_attribute, &query_value, &matcher)
        {
          return true
        }
      }
  }
  match dotted_attrib.len() 
  {
    0 => matcher.is_match(query_value, &attribute.name()),
    _ => matcher.is_match(query_value, &(dotted_attrib + "." + attribute.name())),
  }
}


/**
 *  Match query on a specific attribute `name` on a specific `value` 
 *  both (name and value) having their specific [MatchMethod] 
 *  and attribute `name` use the dotted notation 
 *  attribute:' ' == '' , attribute:w:'' == ''.
 **/
pub fn match_attribute_query(tree: &Tree, nodes : &Vec<TreeNodeId>, name : &str, name_match_type : MatchMethod, value : &str, value_match_type : MatchMethod) -> Result<Vec<TreeNodeId>>
{
  //We reuse the same matcher in every thread (there should be all multithreadable)
  let name_matcher = MatcherMethod::new(&name_match_type, name)?;
  let value_matcher = MatcherMethod::new(&value_match_type, value)?;

  Ok(nodes.par_iter().filter_map(|node_id|
  {
    if let Some(node) = tree.get_node_from_id(*node_id)
    {
      if match_attribute_name_value(&node, &name, &name_matcher, &value, &value_matcher) 
      {
        return Some(*node_id)
      }
    }
    None
  }).collect())
}

fn match_attribute_name_value(node : &Node, query_attr_name : &str, name_matcher : &MatcherMethod, query_attr_value : &str, value_matcher : &MatcherMethod) -> bool
{
  for attribute in node.value().attributes().iter()
  {
    if match_attribute_name_and_value("".into(), &attribute, &query_attr_name, &name_matcher, &query_attr_value, &value_matcher) 
    {
      return true
    }
  }
  return false
}


fn match_attribute_name_and_value(dotted_attrib: String, attribute: &Attribute, query_attr_name : &str, name_matcher : &MatcherMethod, query_attr_value : &str, value_matcher : &MatcherMethod) -> bool
{
  if attribute.type_id() == ValueTypeId::Attributes
  {
      for current_attribute in attribute.value().as_attributes().attributes().iter()
      {
        let dotted_attrib = match dotted_attrib.len() 
        {
          0 => attribute.name().to_string(),
          _ => dotted_attrib.to_string() + "." + attribute.name(),
        };
        if match_attribute_name_and_value(dotted_attrib, current_attribute, query_attr_name, name_matcher, query_attr_value, value_matcher)
        {
          return true;
        }
      }
  }
  else if attribute.type_id() == ValueTypeId::ReflectStruct 
  { 
      //we transform it to attributes 
      let attributes : Vec<Attribute> = attribute.value().as_reflect_struct().attributes(); //XXX inefficent we copy lot of data for nothing
      for current_attribute in attributes.iter() 
      {
        let dotted_attrib = match dotted_attrib.len() 
        {
          0 => attribute.name().to_string(),
          _ => dotted_attrib.to_string() + "." + attribute.name(),
        };
        if match_attribute_name_and_value(dotted_attrib, current_attribute, query_attr_name, name_matcher, query_attr_value, value_matcher)
        {
          return true;
        }
      }

  }
  match dotted_attrib.len() 
  {
    0 => name_matcher.is_match(query_attr_name, &attribute.name()) && 
         value_matcher.is_match(&query_attr_value, &attribute.value().to_string()),
    _ => name_matcher.is_match(query_attr_name, &(dotted_attrib + "." + attribute.name())) && 
         value_matcher.is_match(&query_attr_value, &attribute.value().to_string()),
  }
}

/// Count attributes recursively.
fn attributes_count_rec(value: &Value) -> u64
{
  let mut counter = 0;
  if value.type_id() == ValueTypeId::Attributes
  {
    for current_attribute in value.as_attributes().attributes().iter()
    {
      counter += attributes_count_rec(&current_attribute.value());
    }
  }
  else if value.type_id() == ValueTypeId::ReflectStruct 
  { 
    for attribute in value.as_reflect_struct().attributes().iter()
    {
      counter += attributes_count_rec(&attribute.value());
    }
  }
  else
  {
    counter = 1;
  }
  return counter;
}

/// Count attributes for all [Node] in the [Tree].
pub fn attribute_count(tree : &Tree) -> u64
{
  let nodes = tree.children_rec(None).unwrap();
  nodes.par_iter().map(|node_id|
  {
    if let Some(node) = tree.get_node_from_id(*node_id)
    {
      let mut counter = 0;
      for attribute in node.value().attributes().iter()
      {
        counter += attributes_count_rec(&attribute.value());
      }
      return counter;
    }
    0 

  }).sum()
}

/**
 * Multithread function that search all [Node] in the tree and return the one that have a first-level [Attribute] of type [ValueTypeId::VFileBuilder].
 */
//XXX we should search recursively if attribute contain an other vfiles
pub fn find_vfiles(tree : &Tree) -> Vec<TreeNodeId>
{
  //XXX pass node list
  let nodes = tree.children_rec(None).unwrap();
  nodes.par_iter().filter_map(|node_id|
  {
    if let Some(node) = tree.get_node_from_id(*node_id)
    {
      for attribute in node.value().attributes().iter()
      {
        if attribute.type_id() == ValueTypeId::VFileBuilder
        {
          return Some(*node_id) 
        }
      }
    }
    None
  }).collect()
}
