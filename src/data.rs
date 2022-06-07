//! Method and Struct to search in [Node] [VFile](tap::vfile::VFile) Data [tap::attribute::Attribute].

use tap::tree::{Tree, TreeNodeId};
use tap::node::Node;

use rayon::prelude::*;
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::SearcherBuilder;
use grep_searcher::sinks::Bytes;
use regex::bytes::RegexBuilder;
use anyhow::Result;

/**
 *  Method to search in [Node] data Attribute content. 
 */
pub enum DataMethod
{
  /// Search raw data using regexp.
  Regex,
  /// Search Unicode 8 or Unicode 16 text line by line using regexp.
  Text,
}

pub fn query_data(tree : &Tree, nodes : &Vec<TreeNodeId>, query_value : &str, data_method : DataMethod) -> Result<Vec<TreeNodeId>> 
{
  match data_method
  {
    DataMethod::Regex => query_data_regex(tree, nodes, query_value), 
    DataMethod::Text =>  query_data_line(tree, nodes, query_value),
  }
}

/// Search in `nodes` data if RegEx `query_value` match file content.
/// Use a `RegexBuilder` with unicode, dot_matches_new_line and case_insensitive set to true.
/// Only Unicode 8 and ascii will match, 
/// \x can be use to search for binary data.
pub fn query_data_regex(tree : &Tree, nodes : &Vec<TreeNodeId>, query_value : &str) -> Result<Vec<TreeNodeId>>
{
  let mut builder = RegexBuilder::new(query_value);
  builder.unicode(true);//accept UTF-8 in regex exp,  
  builder.dot_matches_new_line(true);
  builder.case_insensitive(true);
  let query_compiled = builder.build()?;

  Ok(nodes.par_iter().filter_map(|node_id|
  {
     if let Some(node) = tree.get_node_from_id(*node_id)
     {
       if match_data_regex(&node, &query_compiled) 
       {
         return Some(*node_id)
       }
     }
     None
  }).collect())
}

//return false on error so we continue on other nodes
fn match_data_regex(node: &Node, query_compiled : &regex::bytes::Regex) -> bool
{
  let data = match node.value().get_value("data")
  {
    None => return false,
    Some(data) => data,
  };
  let builder = match data.try_as_vfile_builder()
  {
    None => return false,
    Some(builder) => builder,
  };

  let mut file = match builder.open()
  {
    Err(_)=> return false,
    Ok(file) => file,
  };

  let mut buff = [0; 4096];
  let mut readed = 0;
  let file_size = builder.size();

  while readed < file_size
  {
    match file.read(&mut buff)
    {
      Ok(n) => { readed += n as u64; if (n <= 0) {return false} }, 
      Err(_err) => return false,
    };

    let res = query_compiled.is_match(&buff);
    if res == true 
    {
      return true
    }
  }

  false  
}

/**
 *  Search for all `nodes` if RegEx `query_value` match file content.
 *  Search line of text, line by line (search for a '\n' then match on a line),
 *  Line size is limited by heap_limit (1024*1024*100).
 *  It takes a str (utf8) string as argument and search for both utf-8 and utf-16.
 **/
pub fn query_data_line(tree : &Tree, nodes : &Vec<TreeNodeId>, query_value : &str) -> Result<Vec<TreeNodeId>>
{
  let query_compiled = RegexMatcher::new(query_value)?;

  Ok(nodes.par_iter().filter_map(|node_id|
  {
     if let Some(node) = tree.get_node_from_id(*node_id)
     {
       if match_data_line(&node, &query_compiled) 
       {
         return Some(*node_id)
       }
     }
     None
  }).collect())
}

fn match_data_line(node: &Node, query_compiled : &RegexMatcher) -> bool
{
  let data = match node.value().get_value("data")
  {
    None => return false,
    Some(data) => data,
  };
  let builder = match data.try_as_vfile_builder()
  {
    None => return false,
    Some(builder) => builder,
  };

  let file = match builder.open()
  {
    Err(_)=> return false,
    Ok(file) => file,
  };

  //optimize by having one builder , it's slow ...
  let mut searcher_builder = SearcherBuilder::new();
  searcher_builder.heap_limit(Some(1024*1024*100));//will allocate 100M each time 
  let mut searcher = searcher_builder.build(); //reuse it, or create it in query_data_line if possible ?
  let mut matches: Vec<u64> = vec![];

  //could use a UTF8 here too
  let sink = Bytes(|lnum, line| {
    let _match = match query_compiled.find(line) 
    {
        Err(_err) => {}, //{ println!("Error {}", err); }, 
        Ok(_) => matches.push(lnum), 
    };
    Ok(true)
  });

  let _ = searcher.search_reader(&query_compiled, file, sink); //return result and error so we can have more info ? 
  if matches.len() > 0
  {
    return true;
  }

  false
}
