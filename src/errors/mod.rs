//! Fachada pública para os tipos de erro da crate.

//! Cada módulo mantém seus erros junto da implementação que os produz. Este
//! módulo existe apenas como ponto de importação conveniente para consumidores
//! externos.

pub use crate::{assembler::AssemblerError, simulator::SimulationError};
