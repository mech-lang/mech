use crate::program::{Program, ProgramConfig, ProgramEnvironment};
use crossbeam_channel::{Receiver, Sender};
use mech_core::PrettyPrint;

#[derive(Debug, Clone)]
pub enum ClientMessage {
  Ready,
  Ack(String),
  Data(String),
  StepDone,
  Stopped,
  Error(String),
}

#[derive(Debug, Clone)]
pub enum RunLoopMessage {
  Step,
  Stop,
  Load(String),
  Eval(String),
  Configure(ProgramEnvironment),
}

pub struct RunLoop {
  pub outgoing: Sender<RunLoopMessage>,
  pub incoming: Receiver<ClientMessage>,
}

impl RunLoop {
  pub fn send(&self, message: RunLoopMessage) -> Result<(), String> {
    self.outgoing.send(message).map_err(|e| e.to_string())
  }

  pub fn recv(&self) -> Result<ClientMessage, String> {
    self.incoming.recv().map_err(|e| e.to_string())
  }
}

pub struct ProgramRunner {
  config: ProgramConfig,
}

impl ProgramRunner {
  pub fn new(name: &str) -> Self {
    Self { config: ProgramConfig { name: name.to_string(), environment: ProgramEnvironment::default() } }
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
            if let Err(err) = program.compile_program(&source) {
              let _ = tx_evt.send(ClientMessage::Error(err.display_message()));
            } else {
              let _ = tx_evt.send(ClientMessage::Ack("Loaded source".to_string()));
              let _ = tx_evt.send(ClientMessage::StepDone);
            }
          }
          RunLoopMessage::Eval(source) => {
            match program.run_program(&source) {
              Ok(value) => {
                #[cfg(feature = "pretty_print")]
                let _ = tx_evt.send(ClientMessage::Data(value.pretty_print()));
                #[cfg(not(feature = "pretty_print"))]
                let _ = tx_evt.send(ClientMessage::Data(format!("{:#?}", value)));
                let _ = tx_evt.send(ClientMessage::StepDone);
              }
              Err(err) => {
                let _ = tx_evt.send(ClientMessage::Error(err.display_message()));
              }
            }
          }
          RunLoopMessage::Configure(environment) => {
            program.set_environment(environment);
            let _ = tx_evt.send(ClientMessage::Ack("Configured environment".to_string()));
            let _ = tx_evt.send(ClientMessage::StepDone);
          }
          RunLoopMessage::Step => {
            let _ = tx_evt.send(ClientMessage::Ack("Stepped".to_string()));
            let _ = tx_evt.send(ClientMessage::StepDone);
          }
          RunLoopMessage::Stop => {
            let _ = tx_evt.send(ClientMessage::Ack("Stopping".to_string()));
            let _ = tx_evt.send(ClientMessage::StepDone);
            let _ = tx_evt.send(ClientMessage::Stopped);
            break;
          }
        }
      }
    });
    RunLoop { outgoing: tx_cmd, incoming: rx_evt }
  }
}
