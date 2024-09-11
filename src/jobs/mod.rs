use std::{future::Future, sync::Arc, time::Duration};

use poise::serenity_prelude::{self, prelude::TypeMap, Cache, Http};
use tokio::sync::RwLock;

pub mod notice;

#[derive(Clone)]
pub struct JobRunnerContext<S: Clone> {
    pub data: Arc<RwLock<TypeMap>>,
    pub http: Arc<Http>,
    pub cache: Arc<Cache>,
    pub state: Arc<S>,
}

pub trait FnRunner<'a, S: Clone> {
    type Fut: Future<Output = Self::Output> + Send;
    type Output;

    fn call(&mut self, ctx: &'a JobRunnerContext<S>) -> Self::Fut;
}

impl<'a, F, Fut, S: Clone + 'a> FnRunner<'a, S> for F
where
    F: Fn(&'a JobRunnerContext<S>) -> Fut,
    Fut: Future + Send,
{
    type Fut = Fut;
    type Output = Fut::Output;

    fn call(&mut self, ctx: &'a JobRunnerContext<S>) -> Self::Fut {
        (self)(ctx)
    }
}

impl<S: Clone> JobRunnerContext<S> {
    pub async fn execute<F>(&self, mut runner: F)
    where
        F: for<'a> FnRunner<'a, S, Output = ()>,
    {
        runner.call(self).await;
    }
}

pub struct JobRunner<S: Send + Sync + Clone> {
    pub tasks: Vec<tokio::task::JoinHandle<()>>,
    pub ctx: JobRunnerContext<S>,
}

impl<S> JobRunner<S>
where
    S: Send + Sync + Clone + 'static,
{
    pub fn new(client: &serenity_prelude::Client, state: S) -> Self {
        Self {
            tasks: vec![],
            ctx: JobRunnerContext {
                data: client.data.clone(),
                http: client.http.clone(),
                cache: client.cache.clone(),
                state: Arc::new(state),
            },
        }
    }

    pub async fn start<F>(&mut self, runner: F)
    where
        F: Send + Sync + 'static + for<'a> FnRunner<'a, S, Output = ()>,
    {
        let ctx = Arc::new(self.ctx.clone());

        let task = tokio::spawn(async move {
            let ctx = ctx.clone();

            ctx.execute(runner).await;
        });

        self.tasks.push(task);
    }

    pub async fn stop(&mut self) {
        for task in self.tasks.iter() {
            task.abort();
        }
    }
}