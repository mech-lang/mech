use crate::*;
use std::ffi::OsStr;
use bincode::config::standard;

fn list_files(path: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
  if !path.is_dir() {
    // If it's a file, return a vector containing just this path
    return Ok(vec![path.to_path_buf()]);
  }
  
  let mut files = Vec::new();
  for entry in fs::read_dir(path)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      files.extend(list_files(&path)?);
    } else {
      files.push(path);
    }
  }
  Ok(files)
}
  
pub struct MechFileSystem {
  sources: Arc<RwLock<MechSources>>,
  tx: Sender<Event>,                     
  watchers: Vec<Box<dyn Watcher>>,                 
  reload_thread: JoinHandle<()>,                     
}

impl MechFileSystem {

  pub fn new() -> Self {
    let sources = Arc::new(RwLock::new(MechSources::new()));
    let (tx, rx) = unbounded::<Event>();
    let worker_sources = sources.clone();
    let reload_thread = thread::spawn(move || {
      for res in rx {
        match res.kind {
          notify::EventKind::Modify(knd) => {
            for event_path in res.paths {
              match worker_sources.write() {
                Ok(mut sources) => {
                  let canonical_path = event_path.canonicalize().unwrap();
                  println!("{} Loaded: {}", "[Reload]".truecolor(153,221,85), canonical_path.display());
                  sources.reload_source(&canonical_path);
                },
                Err(e) => {
                  println!("watch error: {:?}", e);
                },
              }
            }
          }
          notify::EventKind::Create(_) => (),
          notify::EventKind::Remove(_) => (),
          _ => todo!(),
        }
      }
    });
    MechFileSystem {
      sources,
      tx,
      reload_thread,
      watchers: Vec::new(),
    }
  }

  pub fn set_stylesheet(&mut self, stylesheet: &str) -> MResult<()> {
    match self.sources.write() {
      Ok(mut sources) => {
        sources.set_stylesheet(stylesheet);
        Ok(())
      },
      Err(e) => {
        Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Could not set stylesheet".to_string(), id: line!(), kind: MechErrorKind::None})
      },
    }
  }

  pub fn set_shim(&mut self, shim: &str) -> MResult<()> {
    match self.sources.write() {
      Ok(mut sources) => {
        sources.set_shim(shim);
        Ok(())
      },
      Err(e) => {
        Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Could not set shim".to_string(), id: line!(), kind: MechErrorKind::None})
      },
    }
  }

  pub fn sources(&self) -> Arc<RwLock<MechSources>> {
    self.sources.clone()
  }

  pub fn add_code(&mut self, code: &MechSourceCode) -> MResult<()> {
    {
      match self.sources.write() {
        Ok(mut sources) => {
          sources.add_code(&code)
        },
        Err(e) => {
          Err(MechError{file: file!().to_string(), tokens: vec![], msg: "Could not add code".to_string(), id: line!(), kind: MechErrorKind::None})
        },
      }
    }
  }

  pub fn watch_source(&mut self, src: &str) -> MResult<()> {
    let src_path = Path::new(src.clone());

    // Collect all the files that are in the watched directory
    let files = list_files(&src_path)?;

    {
      match self.sources.write() {
        Ok(mut sources) => {
          for f in files {
            // load mech source code
            if f.extension() == Some(OsStr::new("mec")) || f.extension() == Some(OsStr::new("🤖")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            // load mech bytecode
            } else if f.extension() == Some(OsStr::new("mecb"))  {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            // load mech docs
            } else if f.extension() == Some(OsStr::new("mdoc")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }   
            // load mech config file
            } else if f.extension() == Some(OsStr::new("mpkg")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }              
            // load matlab file           
            } else if f.extension() == Some(OsStr::new("m")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }       
            // load html/css files                  
            } else if f.extension() == Some(OsStr::new("html")) 
                    || f.extension() == Some(OsStr::new("htm"))
                    || f.extension() == Some(OsStr::new("css")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            // load markdown files
            } else if f.extension() == Some(OsStr::new("md")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            // load comma-separated values (csv) files
            } else if f.extension() == Some(OsStr::new("csv")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            // load js files
            } else if f.extension() == Some(OsStr::new("js")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            // load images
            } else if f.extension() == Some(OsStr::new("png")) 
                    || f.extension() == Some(OsStr::new("jpg")) 
                    || f.extension() == Some(OsStr::new("jpeg")) 
                    || f.extension() == Some(OsStr::new("gif")) 
                    || f.extension() == Some(OsStr::new("svg")) {
              match sources.add_source(&f.display().to_string(),src) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
              }
            } else {
              //println!("{} Skipping: {}", "[Skip]".truecolor(153,221,85), f.display());
            }
          }
        }
        Err(e) => {
          return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None});
        },
      }
    }

    let tx = self.tx.clone();

    match notify::recommended_watcher(move |res| {
      if let Ok(event) = res {
        tx.send(event).unwrap();
      }
    }) 
    {
      Ok(mut watcher) => {
        match watcher.watch(&src_path, RecursiveMode::Recursive) {
          Ok(_) => {
            println!("{} Watching: {}", "[Watch]".truecolor(153,221,85), src_path.display());
            self.watchers.push(Box::new(watcher));
          }
          Err(err) => {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Error watching path: {}", err), id: line!(), kind: MechErrorKind::None});
          },
        }       
      }
      Err(err) => println!("[Watch] Error creating watcher: {}", err),
    }
    Ok(())
  }

}

pub struct MechSources {
  index: u64,
  stylesheet: String,
  shim: String,
  sources: HashMap<u64,MechSourceCode>,             // u64 is the hash of the relative source 
  trees: HashMap<u64,MechSourceCode>,               // stores the ast for the sources
  errors: HashMap<u64,Vec<MechError>>,              // stores the errors for the sources
  html: HashMap<u64,MechSourceCode>,                // stores the html for the sources
  pub directory: HashMap<PathBuf, PathBuf>,             // relative source -> absolute source
  reverse_lookup: HashMap<PathBuf, PathBuf>,        // absolute source -> relative source
  id_map: HashMap<u64,PathBuf>,                     // hash -> path
}

impl std::fmt::Debug for MechSources {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MechSources")
    .field("sources", &self.sources)
    .finish()
  }
}
    
impl MechSources {

  pub fn new() -> Self {
    MechSources {
      index: 0,
      stylesheet: "".to_string(),
      shim: "".to_string(),
      sources: HashMap::new(),
      trees: HashMap::new(),
      html: HashMap::new(),
      errors: HashMap::new(),
      directory: HashMap::new(),
      reverse_lookup: HashMap::new(),
      id_map: HashMap::new(),
    }
  }

  pub fn html_iter(&self) -> impl Iterator<Item = (&u64,&MechSourceCode)> {
    self.html.iter()
  }
  
  pub fn sources_iter(&self) -> impl Iterator<Item = (&u64,&MechSourceCode)> {
    self.sources.iter()
  }

  pub fn trees_iter(&self) -> impl Iterator<Item = (&u64,&MechSourceCode)> {
    self.trees.iter()
  }

  pub fn get_path_from_id(&self, id: u64) -> Option<&PathBuf> {
    self.id_map.get(&id)
  }

  pub fn reload_source(&mut self, path: &PathBuf) -> MResult<()> {

    let file_id = hash_str(&path.display().to_string());
    let new_source = read_mech_source_file(&path)?;

    // Get the stale sources
    let mut source = self.sources.get_mut(&file_id).unwrap();
    let mut tree = self.trees.get_mut(&file_id).unwrap();
    let mut html = self.html.get_mut(&file_id).unwrap();

    
    // update the tree
    let (new_tree, new_html) = match source {
      MechSourceCode::String(ref source) => match parser::parse(&source) {
        Ok(tree) => {
          let mut formatter = Formatter::new();
          let mech_html = formatter.format_html(&tree,self.stylesheet.clone(),self.shim.clone());
          (MechSourceCode::Tree(tree), 
            MechSourceCode::Html(mech_html))
        }
        Err(err) => {return Err(err)},
      },
      MechSourceCode::Html(ref html) => {
        // TODO If it's HTML, we can parse it as a Mech source code.
        (MechSourceCode::Tree(core::Program{title: None, body: core::Body{sections: vec![]}}), 
          MechSourceCode::Html(html.clone()))
      },
      _ => {
        todo!("Handle other source formats?");
      }
    };
          
    // update
    *source = new_source;
    *html = new_html;
    *tree = new_tree;
    
    Ok(())
  }

  pub fn set_stylesheet(&mut self, stylesheet: &str) {
    self.stylesheet = stylesheet.to_string();
  }

  pub fn set_shim(&mut self, shim: &str) {
    self.shim = shim.to_string();
  }

  pub fn add_code(&mut self, code: &MechSourceCode) -> MResult<()> {
    
    match code {
      MechSourceCode::String(ref source) => {
        let tree = parser::parse(&source)?;
        let mut formatter = Formatter::new();
        let mech_html = formatter.format_html(&tree,self.stylesheet.clone(),self.shim.clone());
        //let mech_html = Formatter::humanize_html(mech_html);

        // Save all this so we don't have to do it later.
        let file_id = hash_str(&source);
        self.sources.insert(file_id, code.clone());
        self.trees.insert(file_id, MechSourceCode::Tree(tree));
        self.html.insert(file_id, MechSourceCode::Html(mech_html));
        self.id_map.insert(file_id,PathBuf::new());
      },
      MechSourceCode::Tree(ref tree) => {
        let mut formatter = Formatter::new();
        let mech_html = formatter.format_html(&tree,self.stylesheet.clone(),self.shim.clone());
        //let mech_html = Formatter::humanize_html(mech_html);

        // Save all this so we don't have to do it later.
        let file_id = hash_str(&format!("{:?}", tree));
        self.sources.insert(file_id, code.clone());
        self.trees.insert(file_id, code.clone());
        self.html.insert(file_id, MechSourceCode::Html(mech_html));
        self.id_map.insert(file_id,PathBuf::new());
      },
      _ => {
        todo!("Handle other source formats?");
      }
    }
    Ok(())
  }

  fn to_tree_and_html(&mut self, node: &MechSourceCode) -> Result<(MechSourceCode, MechSourceCode), MechError> {
    match node {
      // Raw source text: parse it and format HTML
      MechSourceCode::String(source) => {
        let tree = match parser::parse(source) {
          Ok(t) => t,
          Err(err) => {
            println!("{} {:?}", "[Parse Error]".truecolor(255, 0, 0), err);
            return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "Failed to parse source code".to_string(),id: line!(),kind: MechErrorKind::None,});
          }
        };
        let mut formatter = Formatter::new();
        let mech_html = formatter.format_html(&tree, self.stylesheet.clone(),self.shim.clone());
        Ok((MechSourceCode::Tree(tree), MechSourceCode::Html(mech_html)))
      }
      MechSourceCode::Program(code_vec) => {
        let mut combined = core::Program {
          title: None,
          body: core::Body { sections: vec![] },
        };
        let mut combined_html = "".to_string();
        for child in code_vec {
          let (child_tree_sc, child_html_sc) = self.to_tree_and_html(child)?;
          if let MechSourceCode::Tree(child_prog) = child_tree_sc {
            combined.body.sections.extend(child_prog.body.sections.into_iter());
          }
          if let MechSourceCode::Html(h) = child_html_sc {
            combined_html.push_str(&h);
          }
        }
        Ok((MechSourceCode::Tree(combined), MechSourceCode::Html(combined_html)))
      }
      MechSourceCode::Html(html) => Ok((MechSourceCode::Tree(core::Program {title: None,body: core::Body { sections: vec![] },}),MechSourceCode::Html(html.clone()))),
      MechSourceCode::Tree(t) => {
        let mut formatter = Formatter::new();
        let mech_html = formatter.format_html(t, self.stylesheet.clone(),self.shim.clone());
        Ok((MechSourceCode::Tree(t.clone()), MechSourceCode::Html(mech_html)))
      }
      _ => Ok((MechSourceCode::Tree(core::Program {title: None,body: core::Body { sections: vec![] },}),MechSourceCode::Html("".to_string()))),
    }
  }

  pub fn add_source(&mut self, src_str: &str, src_root: &str) -> MResult<MechSourceCode> {
    use MechSourceCode::*;
    let src_path = std::path::Path::new(src_str);
    let canonical_path = src_path.canonicalize().unwrap();
    let canonical_root = std::path::Path::new(src_root).canonicalize().unwrap();
    let relative_path = match canonical_path.strip_prefix(&canonical_root) {
      Ok(p) => p,
      Err(_) => canonical_path.as_path(),
    };
    match read_mech_source_file(&canonical_path) {
      Ok(MechSourceCode::Image(extension, img_bytes)) => {
        let file_id = hash_str(&canonical_path.display().to_string());

        self.directory
          .insert(relative_path.to_path_buf(), canonical_path.clone());
        self.reverse_lookup
          .insert(canonical_path.clone(), relative_path.to_path_buf());
        self.sources.insert(file_id, MechSourceCode::Image(extension.clone(), img_bytes.clone()));
        self.id_map.insert(file_id, relative_path.to_path_buf());

        let relative_path_str = relative_path.display().to_string();
        let src_path_hash = hash_str(&relative_path_str);

        Ok(MechSourceCode::Image(extension, img_bytes))
      } 
      Ok(src) => {
        let (tree_sc, html_sc) = self.to_tree_and_html(&src)?;
        let file_id = hash_str(&canonical_path.display().to_string());

        self.directory
          .insert(relative_path.to_path_buf(), canonical_path.clone());
        self.reverse_lookup
          .insert(canonical_path.clone(), relative_path.to_path_buf());
        self.sources.insert(file_id, src.clone());
        self.trees.insert(file_id, tree_sc);
        self.html.insert(file_id, html_sc);
        self.id_map.insert(file_id, relative_path.to_path_buf());

        let relative_path_str = relative_path.display().to_string();
        let src_path_hash = hash_str(&relative_path_str);

        if self.index == 0 {
          self.index = file_id;
        } else if src_path_hash == hash_str("index.mec")
          || src_path_hash == hash_str("index.html")
          || src_path_hash == hash_str("index.md")
        {
          self.index = file_id;
        }
        Ok(src)
      }
      Err(err) => Err(err),
    }
  }

  pub fn contains(&self, src: &str) -> bool {
    let src_path = Path::new(src);
    if self.directory.contains_key(src_path) {
      return true;
    } else if self.reverse_lookup.contains_key(src_path) {
      return true;
    } else {
      return false;
    }
  }

  pub fn get_source(&self, src: &str) -> Option<MechSourceCode> {
    if src == "" {
      let file_id = self.index;
      return match self.sources.get(&file_id) {
        Some(code) => Some(code.clone()),
        None => None,
      };
    }
    let absolute_path = self.directory.get(Path::new(src));
    match absolute_path {
      Some(path) => {
        let file_id = hash_str(&path.display().to_string());
        match self.sources.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => None,
        }
      },
      None => {
        let file_id = hash_str(&src);
        match self.sources.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => None,
        }
      },
    }
  }

  pub fn get_tree(&self, src: &str) -> Option<MechSourceCode> {
    if src == "" {
      let file_id = self.index;
      return match self.trees.get(&file_id) {
        Some(code) => Some(code.clone()),
        None => None,
      };
    }
    let absolute_path = self.directory.get(Path::new(src));
    match absolute_path {
      Some(path) => {
        let file_id = hash_str(&path.display().to_string());
        match self.trees.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => None,
        }
      },
      None => {
        let file_id = hash_str(&src);
        match self.trees.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => None,
        }
      },
    }
  }

  pub fn get_image(&self, src: &str) -> Option<MechSourceCode> {
    match self.directory.get(Path::new(src)) {
      Some(path) => {
        let file_id = hash_str(&path.display().to_string());
        match self.sources.get(&file_id) {
          Some(code) => {
            match code {
              MechSourceCode::Image(_, _) => Some(code.clone()),
              _ => None,
            }
          },
          None => None,
        }
      },
      None => None,
    }
  }

  pub fn get_html(&self, src: &str) -> Option<MechSourceCode> {
    if src == "" {
      let file_id = self.index;
      return match self.html.get(&file_id) {
        Some(code) => Some(code.clone()),
        None => None,
      };
    }
    match self.directory.get(Path::new(src)) {
      Some(path) => {
        let file_id = hash_str(&path.display().to_string());
        match self.html.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => None,
        }
      },
      None => {
        let file_id = hash_str(&src);
        match self.html.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => {
            // replace file extension with .mec and search for it again
            let new_src = Path::new(src).with_extension("mec");
            match self.directory.get(&new_src) {
              Some(path) => {
                let file_id = hash_str(&path.display().to_string());
                match self.html.get(&file_id) {
                  Some(code) => Some(code.clone()),
                  None => None,
                }
              }
              None => None,
            }
          },
        }
      },
    }
  }

  pub fn read_mech_files(&mut self, mech_paths: &Vec<String>) -> MResult<Vec<(String,MechSourceCode)>> {
    let mut code: Vec<(String,MechSourceCode)> = Vec::new();
    for path_str in mech_paths {
      let path = Path::new(path_str);
      // Compile a .mec file on the web
      if path_str.starts_with("https") || path_str.starts_with("http") {
        println!("{} {}", "[Downloading]".truecolor(153,221,85), path.display());
        match reqwest::blocking::get(path_str) {
          Ok(response) => {
            match response.text() {
              Ok(text) => {
                let src = MechSourceCode::String(text);

                code.push((path_str.to_owned(), src));
              },
              _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None});},
            }
          }
          _ => {return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None});},
        }
      } else {
        match read_mech_source_file(path) {
          Ok(src) => {
            code.push((path_str.to_owned(), src));
          },
          Err(err) => {
            return Err(err);
          },
        }
      };
    }
    Ok(code)
  }
  
}
  
pub fn read_mech_source_file(path: &Path) -> MResult<MechSourceCode> {
  match path.extension() {
    Some(extension) => {
      match extension.to_str() {
        Some("mecb") => {
          let path = PathBuf::from(path);
          let data = std::fs::read(&path)?;
          let program = load_program_from_file(path)?;   
          Ok(MechSourceCode::ByteCode(program.to_bytes()?))
        }
        Some("mec") | Some("🤖") => {
          match File::open(path) {
            Ok(mut file) => {
              //println!("{} {}", "[Loading]".truecolor(153,221,85), path.display());
              let mut buffer = String::new();
              file.read_to_string(&mut buffer);
              Ok(MechSourceCode::String(buffer))
            }
            Err(err) => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
          }
        }
        Some("html") | Some("htm") | Some("md") | Some("css") => {
          match File::open(path) {
            Ok(mut file) => {
              //println!("{} {}", "[Loading]".truecolor(153,221,85), path.display());
              let mut buffer = String::new();
              file.read_to_string(&mut buffer);
              Ok(MechSourceCode::Html(buffer))
            }
            Err(err) => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
          }
        }
        // handle images
        Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => {
          match File::open(path) {
            Ok(mut file) => {
              //println!("{} {}", "[Loading]".truecolor(153,221,85), path.display());
              let mut buffer = Vec::new();
              file.read_to_end(&mut buffer);
              // store extension and bytes
              let extension = path.extension().and_then(OsStr::to_str).unwrap_or("").to_string();
              Ok(MechSourceCode::Image(extension, buffer))
            }
            Err(err) => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
          }
        }
        Some("csv") => {
          match File::open(path) {
            Ok(mut file) => {
              //println!("{} {}", "[Loading]".truecolor(153,221,85), path.display());
              let mut buffer = String::new();
              let mut rdr = csv::Reader::from_reader(file);
              for result in rdr.records() {
                println!("{:?}", result);
              }
              todo!();
            }
            Err(err) => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::None}),
          }
        }
        x => {
          Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("Unknown file extension: {:?}", x), id: line!(), kind: MechErrorKind::None})
        },
      }
    },
    err => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::GenericError(format!("{:?}", err))}),
  }
}