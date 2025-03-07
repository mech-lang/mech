use crate::*;

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
              match sources.add_source(&f.display().to_string()) {
                Ok(_) => {
                  println!("{} Loaded: {}", "[Load]".truecolor(153,221,85), f.display());
                },
                Err(e) => {
                  return Err(e);
                },
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
          println!("{} Watching: {}", "[Watch]".truecolor(153,221,85), src_path.display());
          watcher.watch(&src_path, RecursiveMode::Recursive).unwrap();
          self.watchers.push(Box::new(watcher));
        }
        Err(err) => println!("[Watch] Error creating watcher: {}", err),
      }
      Ok(())
    }
  
  }
  
  pub struct MechSources {
    index: u64,
    stylesheet: String,
    sources: HashMap<u64,MechSourceCode>,             // u64 is the hash of the relative source 
    trees: HashMap<u64,MechSourceCode>,               // stores the ast for the sources
    errors: HashMap<u64,Vec<MechError>>,              // stores the errors for the sources
    html: HashMap<u64,String>,                        // stores the html for the sources
    directory: HashMap<PathBuf, PathBuf>,             // relative source -> absolute source
    reverse_lookup: HashMap<PathBuf, PathBuf>,        // absolute source -> relative source
    id_map: HashMap<u64,PathBuf>,                     // hash -> path
  }
  
  impl MechSources {
  
    pub fn new() -> Self {
      MechSources {
        index: 0,
        stylesheet: "".to_string(),
        sources: HashMap::new(),
        trees: HashMap::new(),
        html: HashMap::new(),
        errors: HashMap::new(),
        directory: HashMap::new(),
        reverse_lookup: HashMap::new(),
        id_map: HashMap::new(),
      }
    }
  
    pub fn html_iter(&self) -> impl Iterator<Item = (&u64,&String)> {
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
      let new_tree = match source {
        MechSourceCode::String(ref source) => match parser::parse(&source) {
          Ok(tree) => tree,
          Err(err) => {
            todo!("Handle parse error");
          }
        },
        _ => {
          todo!("Handle other source formats?");
        }
      };
      
      // update the html
      let mut formatter = Formatter::new();
      let formatted_mech = formatter.format_html(&new_tree,self.stylesheet.clone());
      let mech_html = Formatter::humanize_html(formatted_mech);
      
      // update
      *source = new_source;
      *html = mech_html;
      *tree = MechSourceCode::Tree(new_tree);
      
      Ok(())
    }
  
    pub fn set_stylesheet(&mut self, stylesheet: &str) {
      self.stylesheet = stylesheet.to_string();
    }
  
    pub fn add_code(&mut self, code: &MechSourceCode) -> MResult<()> {
      
      match code {
        MechSourceCode::String(ref source) => {
          let tree = parser::parse(&source)?;
          let mut formatter = Formatter::new();
          let formatted_mech = formatter.format_html(&tree,self.stylesheet.clone());
          let mech_html = Formatter::humanize_html(formatted_mech);
  
          // Save all this so we don't have to do it later.
          let file_id = hash_str(&source);
          self.sources.insert(file_id, code.clone());
          self.trees.insert(file_id, MechSourceCode::Tree(tree));
          self.html.insert(file_id, mech_html);
          self.id_map.insert(file_id,PathBuf::new());
        },
        MechSourceCode::Tree(ref tree) => {
          let mut formatter = Formatter::new();
          let formatted_mech = formatter.format_html(&tree,self.stylesheet.clone());
          let mech_html = Formatter::humanize_html(formatted_mech);
  
          // Save all this so we don't have to do it later.
          let file_id = hash_str(&format!("{:?}", tree));
          self.sources.insert(file_id, code.clone());
          self.trees.insert(file_id, code.clone());
          self.html.insert(file_id, mech_html);
          self.id_map.insert(file_id,PathBuf::new());
        },
        _ => {
          todo!("Handle other source formats?");
        }
      }
      Ok(())
    }
  
    pub fn add_source(&mut self, src: &str) -> MResult<MechSourceCode> {
      let src_path = Path::new(src);
      let id = hash_str(&src_path.display().to_string());
      let canonical_path = src_path.canonicalize().unwrap();
      self.directory.insert(src_path.to_path_buf(),canonical_path.clone());
      self.reverse_lookup.insert(canonical_path.clone(),src_path.to_path_buf());
      let file_id = hash_str(&canonical_path.display().to_string());
      match read_mech_source_file(src_path) {
        Ok(src) => {
          let tree = match src {
            MechSourceCode::String(ref source) => match parser::parse(&source) {
              Ok(tree) => tree,
              Err(err) => {
                todo!("Handle parse error");
              }
            },
            _ => {
              todo!("Handle other source formats?");
            }
          };
  
          let mut formatter = Formatter::new();
          let formatted_mech = formatter.format_html(&tree,self.stylesheet.clone());
          let mech_html = Formatter::humanize_html(formatted_mech);
  
          // Save all this so we don't have to do it later.
          self.sources.insert(file_id, src.clone());
          self.trees.insert(file_id, MechSourceCode::Tree(tree));
          self.html.insert(file_id, mech_html);
          self.id_map.insert(file_id,src_path.to_path_buf());
  
  
          if self.index == 0 {
            self.index = file_id;
          } else if id == hash_str("index.mec") || id == hash_str("index.html") || id == hash_str("index.md") {
            self.index = file_id;
          }
  
          return Ok(src); 
        },
        Err(err) => {
          return Err(err);
        },
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
  
    pub fn get_html(&self, src: &str) -> Option<String> {
      if src == "" {
        let file_id = self.index;
        return match self.html.get(&file_id) {
          Some(code) => Some(code.clone()),
          None => None,
        };
      }
      let absolute_path = self.directory.get(Path::new(src));
      match absolute_path {
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
            None => None,
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
          /*Some("blx") => {
            match File::open(name) {
              Ok(file) => {
                println!("{} {}", "[Loading]".truecolor(153,221,85), name);
                let mut reader = BufReader::new(file);
                let mech_code: Result<MechSourceCode, bincode::Error> = bincode::deserialize_from(&mut reader);
                match mech_code {
                  Ok(c) => {code.push((name.to_string(),c));},
                  Err(err) => {
                    return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1247, kind: MechErrorKind::GenericError(format!("{:?}", err))});
                  },
                }
              }
              Err(err) => {
                return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: 1248, kind: MechErrorKind::None});
              },
            };
          }*/
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