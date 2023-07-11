mod rpc;

use std::net::SocketAddr;

pub struct ProxyNode {
    pub bind: SocketAddr,
    pub external: SocketAddr,
}

pub struct ProxyOptions {
    pub nodes: Vec<ProxyNode>,
}

pub struct Proxy {
    options: ProxyOptions,
}

impl Proxy {
    pub fn new(options: ProxyOptions) -> Self {
        Self {
            options,
        }
    }

    pub fn in_proxy_node(&self, addr: &SocketAddr) -> bool {
        self.options.nodes.iter().any(|n| &n.external == addr)
    }

    pub fn create_permission(&self, peer: &SocketAddr) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_work() {
        let proxy = Proxy::new(ProxyOptions {
            nodes: vec![ProxyNode {
                bind: "127.0.0.1:3478".parse().unwrap(),
                external: "127.0.0.1:3478".parse().unwrap(),
            }],
        });

        proxy.create_permission(&"127.0.0.1:57210".parse().unwrap());
    }
}
