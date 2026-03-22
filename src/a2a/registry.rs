use crate::a2a::port_pool::PortPool;
use crate::agents::AgentProfile;
use std::collections::HashMap;

pub type AgentId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Idle,
    Busy,
    Done,
}

#[derive(Debug)]
pub struct AgentInstance {
    pub id: AgentId,
    pub profile: String,
    pub port: u16,
    pub url: String,
    pub status: AgentStatus,
}

#[derive(Debug)]
pub struct AgentRegistry {
    pools: HashMap<String, PortPool>,
    instances: HashMap<AgentId, AgentInstance>,
    next_id: AgentId,
}

impl AgentRegistry {
    /// Build registry from profiles, seeding per-profile port pools from a2a blocks.
    pub fn new(profiles: &[AgentProfile]) -> Self {
        let mut pools = HashMap::new();
        for profile in profiles {
            if let Some(a2a) = &profile.a2a {
                let range = a2a.port_range[0]..=a2a.port_range[1];
                pools.insert(profile.name.clone(), PortPool::new(range));
            }
        }
        Self {
            pools,
            instances: HashMap::new(),
            next_id: 1,
        }
    }

    /// Allocate a port and register a new instance slot.
    /// Returns `(port, id, url)`. Caller is responsible for starting the server.
    pub fn reserve_new(&mut self, profile: &str) -> anyhow::Result<(u16, AgentId, String)> {
        let pool = self
            .pools
            .get_mut(profile)
            .ok_or_else(|| anyhow::anyhow!("No port pool configured for profile '{}'", profile))?;
        let port = pool
            .acquire()
            .ok_or_else(|| anyhow::anyhow!("No ports available for profile '{}'", profile))?;
        let id = self.next_id;
        self.next_id += 1;
        let url = format!("http://127.0.0.1:{}", port);
        self.instances.insert(
            id,
            AgentInstance {
                id,
                profile: profile.to_string(),
                port,
                url: url.clone(),
                status: AgentStatus::Busy,
            },
        );
        Ok((port, id, url))
    }

    /// Find an existing idle instance for a profile and mark it busy.
    pub fn try_acquire_idle(&mut self, profile: &str) -> Option<(AgentId, String)> {
        let instance = self
            .instances
            .values_mut()
            .find(|i| i.profile == profile && i.status == AgentStatus::Idle)?;
        instance.status = AgentStatus::Busy;
        Some((instance.id, instance.url.clone()))
    }

    pub fn mark_busy(&mut self, id: AgentId) {
        if let Some(instance) = self.instances.get_mut(&id) {
            instance.status = AgentStatus::Busy;
        }
    }

    /// Mark instance as idle so it can handle the next request.
    pub fn release_instance(&mut self, id: AgentId) {
        if let Some(instance) = self.instances.get_mut(&id) {
            instance.status = AgentStatus::Idle;
        }
    }

    /// Remove instance and return its port to the pool (used when shutting down a server).
    pub fn retire_instance(&mut self, id: AgentId) {
        if let Some(instance) = self.instances.remove(&id) {
            if let Some(pool) = self.pools.get_mut(&instance.profile) {
                pool.release(instance.port);
            }
        }
    }
}
