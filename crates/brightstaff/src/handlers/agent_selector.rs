use std::collections::HashMap;
use std::sync::Arc;

use common::configuration::{
    Agent, AgentFilterChain, Listener, ModelUsagePreference, RoutingPreference,
};
use hermesllm::apis::openai::Message;
use tracing::{debug, warn};

use crate::router::llm_router::RouterService;

/// Errors that can occur during agent selection
#[derive(Debug, thiserror::Error)]
pub enum AgentSelectionError {
    #[error("Listener not found for name: {0}")]
    ListenerNotFound(String),
    #[error("No agents configured for listener: {0}")]
    NoAgentsConfigured(String),
    #[error("Routing service error: {0}")]
    RoutingError(String),
    #[error("Default agent not found for listener: {0}")]
    DefaultAgentNotFound(String),
    #[error("MCP client error: {0}")]
    McpError(String),
}

/// Service for selecting agents based on routing preferences and listener configuration
pub struct AgentSelector {
    router_service: Arc<RouterService>,
}

impl AgentSelector {
    pub fn new(router_service: Arc<RouterService>) -> Self {
        Self {
            router_service,
        }
    }

    /// Find listener by name from the request headers
    pub async fn find_listener(
        &self,
        listener_name: Option<&str>,
        listeners: &[common::configuration::Listener],
    ) -> Result<Listener, AgentSelectionError> {
        let listener = listeners
            .iter()
            .find(|l| listener_name.map(|name| l.name == name).unwrap_or(false))
            .cloned()
            .ok_or_else(|| {
                AgentSelectionError::ListenerNotFound(
                    listener_name.unwrap_or("unknown").to_string(),
                )
            })?;

        Ok(listener)
    }

    /// Create agent name to agent mapping for efficient lookup
    pub fn create_agent_map(&self, agents: &[Agent]) -> HashMap<String, Agent> {
        agents
            .iter()
            .map(|agent| (agent.id.clone(), agent.clone()))
            .collect()
    }

    /// Select appropriate agent based on routing preferences
    pub async fn select_agent(
        &self,
        messages: &[Message],
        listener: &Listener,
        trace_parent: Option<String>,
    ) -> Result<AgentFilterChain, AgentSelectionError> {
        let agents = listener
            .agents
            .as_ref()
            .ok_or_else(|| AgentSelectionError::NoAgentsConfigured(listener.name.clone()))?;

        // If only one agent, skip routing
        if agents.len() == 1 {
            debug!("Only one agent available, skipping routing");
            return Ok(agents[0].clone());
        }

        let usage_preferences = self
            .convert_agent_description_to_routing_preferences(agents)
            .await;
        debug!(
            "Agents usage preferences for agent routing str: {}",
            serde_json::to_string(&usage_preferences).unwrap_or_default()
        );

        match self
            .router_service
            .determine_route(messages, trace_parent, Some(usage_preferences))
            .await
        {
            Ok(Some((_, agent_name))) => {
                debug!("Determined agent: {}", agent_name);
                let selected_agent = agents
                    .iter()
                    .find(|a| a.id == agent_name)
                    .cloned()
                    .ok_or_else(|| {
                        AgentSelectionError::RoutingError(format!(
                            "Selected agent '{}' not found in listener agents",
                            agent_name
                        ))
                    })?;
                Ok(selected_agent)
            }
            Ok(None) => {
                debug!("No agent determined using routing preferences, using default agent");
                self.get_default_agent(agents, &listener.name)
            }
            Err(err) => Err(AgentSelectionError::RoutingError(err.to_string())),
        }
    }

    /// Get the default agent or the first agent if no default is specified
    fn get_default_agent(
        &self,
        agents: &[AgentFilterChain],
        listener_name: &str,
    ) -> Result<AgentFilterChain, AgentSelectionError> {
        agents
            .iter()
            .find(|a| a.default.unwrap_or(false))
            .cloned()
            .or_else(|| {
                warn!(
                    "No default agent found, routing request to first agent: {}",
                    agents[0].id
                );
                Some(agents[0].clone())
            })
            .ok_or_else(|| AgentSelectionError::DefaultAgentNotFound(listener_name.to_string()))
    }

    /// Convert agent descriptions to routing preferences
    async fn convert_agent_description_to_routing_preferences(
        &self,
        agents: &[AgentFilterChain],
    ) -> Vec<ModelUsagePreference> {
        let mut preferences = Vec::new();

        for agent_chain in agents {
            preferences.push(ModelUsagePreference {
                model: agent_chain.id.clone(),
                routing_preferences: vec![RoutingPreference {
                    name: agent_chain.id.clone(),
                    description: agent_chain.description.clone().unwrap_or_default(),
                }],
            });
        }

        preferences
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::configuration::{AgentFilterChain, Listener};

    fn create_test_router_service() -> Arc<RouterService> {
        Arc::new(RouterService::new(
            vec![], // empty providers for testing
            "http://localhost:8080".to_string(),
            "test-model".to_string(),
            "test-provider".to_string(),
        ))
    }

    fn create_test_agent(name: &str, description: &str, is_default: bool) -> AgentFilterChain {
        AgentFilterChain {
            id: name.to_string(),
            description: Some(description.to_string()),
            default: Some(is_default),
            filter_chain: vec![name.to_string()],
        }
    }

    fn create_test_listener(name: &str, agents: Vec<AgentFilterChain>) -> Listener {
        Listener {
            name: name.to_string(),
            agents: Some(agents),
            port: 8080,
            router: None,
        }
    }

    fn create_test_agent_struct(name: &str) -> Agent {
        Agent {
            id: name.to_string(),
            agent_type: Some("test".to_string()),
            url: "http://localhost:8080".to_string(),
            tool: None,
            transport: None,
        }
    }

    #[tokio::test]
    async fn test_find_listener_success() {
        let router_service = create_test_router_service();
        let selector = AgentSelector::new(router_service);

        let listener1 = create_test_listener("test-listener", vec![]);
        let listener2 = create_test_listener("other-listener", vec![]);
        let listeners = vec![listener1.clone(), listener2];

        let result = selector
            .find_listener(Some("test-listener"), &listeners)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "test-listener");
    }

    #[tokio::test]
    async fn test_find_listener_not_found() {
        let router_service = create_test_router_service();
        let selector = AgentSelector::new(router_service);

        let listeners = vec![create_test_listener("other-listener", vec![])];

        let result = selector
            .find_listener(Some("nonexistent"), &listeners)
            .await;

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            AgentSelectionError::ListenerNotFound(_)
        );
    }

    #[test]
    fn test_create_agent_map() {
        let router_service = create_test_router_service();
        let selector = AgentSelector::new(router_service);

        let agents = vec![
            create_test_agent_struct("agent1"),
            create_test_agent_struct("agent2"),
        ];

        let agent_map = selector.create_agent_map(&agents);

        assert_eq!(agent_map.len(), 2);
        assert!(agent_map.contains_key("agent1"));
        assert!(agent_map.contains_key("agent2"));
    }

    #[tokio::test]
    async fn test_convert_agent_description_to_routing_preferences() {
        let router_service = create_test_router_service();
        let selector = AgentSelector::new(router_service);

        let agents = vec![
            create_test_agent("agent1", "First agent description", true),
            create_test_agent("agent2", "Second agent description", false),
        ];

        let preferences = selector
            .convert_agent_description_to_routing_preferences(&agents)
            .await;

        assert_eq!(preferences.len(), 2);
        assert_eq!(preferences[0].model, "agent1");
        assert_eq!(preferences[0].routing_preferences[0].name, "agent1");
        assert_eq!(
            preferences[0].routing_preferences[0].description,
            "First agent description"
        );
    }

    #[test]
    fn test_get_default_agent() {
        let router_service = create_test_router_service();
        let selector = AgentSelector::new(router_service);

        let agents = vec![
            create_test_agent("agent1", "First agent", false),
            create_test_agent("agent2", "Default agent", true),
            create_test_agent("agent3", "Third agent", false),
        ];

        let result = selector.get_default_agent(&agents, "test-listener");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "agent2");
    }

    #[test]
    fn test_get_default_agent_fallback_to_first() {
        let router_service = create_test_router_service();
        let selector = AgentSelector::new(router_service);

        let agents = vec![
            create_test_agent("agent1", "First agent", false),
            create_test_agent("agent2", "Second agent", false),
        ];

        let result = selector.get_default_agent(&agents, "test-listener");

        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "agent1");
    }
}
