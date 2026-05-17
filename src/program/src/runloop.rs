use crate::program::{Program, ProgramConfig};
use crossbeam_channel::{Receiver, Sender};

#[derive(Debug, Clone)]
pub enum ClientMessage {
  Ready,
  StepDone,
  Stopped,
  Error(String),
}

#[derive(Debug, Clone)]
pub enum RunLoopMessage {
  Step,
  Stop,
  Load(String),
}

pub struct RunLoop {
  pub outgoing: Sender<RunLoopMessage>,
  pub incoming: Receiver<ClientMessage>,
}

pub struct ProgramRunner {
  config: ProgramConfig,
}

impl ProgramRunner {
  pub fn new(name: &str) -> Self {
    Self { config: ProgramConfig { name: name.to_string(), ..ProgramConfig::default() } }
  }

  pub fn run(self) -> RunLoop {
    let (tx_cmd, rx_cmd) = crossbeam_channel::unbounded();
    let (tx_evt, rx_evt) = crossbeam_channel::unbounded();
    std::thread::spawn(move || {
      let mut program = Program::new(self.config);
      let _ = tx_evt.send(ClientMessage::Ready);
      while let Ok(msg) = rx_cmd.recv() {
        match msg {
          RunLoopMessage::Load(source) => {
            if let Err(err) = program.run_program(&source) {
              let _ = tx_evt.send(ClientMessage::Error(err.to_string()));
            } else {
              let _ = tx_evt.send(ClientMessage::StepDone);
            }
          }
          RunLoopMessage::Step => {
            let _ = tx_evt.send(ClientMessage::StepDone);
          }
          RunLoopMessage::Stop => {
            let _ = tx_evt.send(ClientMessage::Stopped);
            break;
          }
        }
      }
    });
    RunLoop { outgoing: tx_cmd, incoming: rx_evt }
  }
}
