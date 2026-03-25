use anyhow::Result;
use rquickjs::{context::EvalOptions, AsyncContext, AsyncRuntime, CatchResultExt};
use tracing::debug;

const MAX_STACK_SIZE: usize = 512 * 1024;
const MAX_MEMORY: usize = 64 * 1024 * 1024;

pub struct JsRuntime {
    pub rt: AsyncRuntime,
    pub ctx: AsyncContext,
}

impl JsRuntime {
    pub async fn new() -> Result<Self> {
        let rt = AsyncRuntime::new()?;
        rt.set_max_stack_size(MAX_STACK_SIZE).await;
        rt.set_memory_limit(MAX_MEMORY).await;

        let ctx = AsyncContext::full(&rt).await?;
        debug!("QuickJS runtime initialized (stack={}KB, mem={}MB)", MAX_STACK_SIZE / 1024, MAX_MEMORY / 1024 / 1024);
        Ok(Self { rt, ctx })
    }

    pub async fn eval(&self, code: &str, filename: &str) -> Result<()> {
        let code = code.to_string();
        let filename = filename.to_string();
        self.ctx
            .with(|ctx| {
                let result: rquickjs::Result<()> = (|| {
                    let mut opts = EvalOptions::default();
                    opts.filename = Some(filename.clone());
                    ctx.eval_with_options::<(), _>(code.as_str(), opts)?;
                    Ok(())
                })();
                result.catch(&ctx).map_err(|e| anyhow::anyhow!("{e:?}"))
            })
            .await?;
        Ok(())
    }

    pub async fn execute_pending_jobs(&self) {
        let _ = self.rt.execute_pending_job().await;
    }
}
