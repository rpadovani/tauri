// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! The singleton async runtime used by Tauri and exposed to consumers.
//! Wraps a `tokio` Runtime and is meant to be used by initialization code, such as plugins `initialization` and app `setup` hooks.
//! Fox more complex use cases, consider creating your own runtime.
//! For command handlers, it's recommended to use a plain `async fn` command.

use futures_lite::future::FutureExt;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
pub use tokio::{
  runtime::Handle,
  sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex, RwLock,
  },
  task::JoinHandle as TokioJoinHandle,
};

use std::{
  fmt,
  future::Future,
  pin::Pin,
  task::{Context, Poll},
};

static RUNTIME: OnceCell<Runtime> = OnceCell::new();

/// An owned permission to join on a task (await its termination).
#[derive(Debug)]
pub struct JoinHandle<T>(TokioJoinHandle<T>);

impl<T> Future for JoinHandle<T> {
  type Output = crate::Result<T>;
  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self
      .0
      .poll(cx)
      .map_err(|e| crate::Error::JoinError(Box::new(e)))
  }
}

/// Runtime handle definition.
pub trait RuntimeHandle: fmt::Debug + Clone + Sync + Sync {
  /// Spawn a future onto the runtime.
  fn spawn<F: Future>(&self, task: F) -> JoinHandle<F::Output>
  where
    F: Future + Send + 'static,
    F::Output: Send + 'static;

  /// Run a future to completion on runtime.
  fn block_on<F: Future>(&self, task: F) -> F::Output;
}

impl RuntimeHandle for Handle {
  fn spawn<F: Future>(&self, task: F) -> JoinHandle<F::Output>
  where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
  {
    JoinHandle(self.spawn(task))
  }

  fn block_on<F: Future>(&self, task: F) -> F::Output {
    self.block_on(task)
  }
}

/// Returns a handle to the async runtime.
pub fn handle() -> impl RuntimeHandle {
  let runtime = RUNTIME.get_or_init(|| Runtime::new().unwrap());
  runtime.handle().clone()
}

/// Run a future to completion on runtime.
pub fn block_on<F: Future>(task: F) -> F::Output {
  let runtime = RUNTIME.get_or_init(|| Runtime::new().unwrap());
  runtime.block_on(task)
}

/// Spawn a future onto the runtime.
pub fn spawn<F>(task: F) -> JoinHandle<F::Output>
where
  F: Future + Send + 'static,
  F::Output: Send + 'static,
{
  let runtime = RUNTIME.get_or_init(|| Runtime::new().unwrap());
  JoinHandle(runtime.spawn(task))
}

#[cfg(test)]
mod tests {
  use super::*;
  #[tokio::test]
  async fn handle_spawn() {
    let handle = handle();
    let join = handle.spawn(async { 5 });
    assert_eq!(join.await.unwrap(), 5);
  }

  #[test]
  fn handle_block_on() {
    let handle = handle();
    assert_eq!(handle.block_on(async { 0 }), 0);
  }
}
