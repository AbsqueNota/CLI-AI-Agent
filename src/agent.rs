pub mod runtime;

use anyhow::Result;
use rig::agent::{Agent, AgentBuilder};
use crate::{
    models::{
        Model,
    }, 
    provider::LlmProvider,
};

pub struct AgentFactory;

impl AgentFactory {
    pub fn build(provider: &Box<dyn LlmProvider>, model: &Model) -> Result<Agent> {
        let dyn_model = provider.native_completion(&model.id)?;
        let mut builder = AgentBuilder::new(dyn_model)
            .name(&model.name)
            .preamble(&model.preamble);

        let settings = &model.settings;
        
        macro_rules! set_opt {
            ($builder:ident, $opt:expr, $method:ident) => {
                if let Some(val) = $opt {
                    $builder = $builder.$method(val);
                }
            };
        }
        set_opt!(builder, settings.max_tokens,  max_tokens);
        set_opt!(builder, settings.temperature, temperature);
        set_opt!(builder, settings.top_p,       top_p);
        set_opt!(builder, settings.seed,        seed);
        set_opt!(builder, &settings.stop,       stop);
        set_opt!(builder, provider.to_additional_params(model), additional_params);
        
        Ok(builder.build())
    }
}