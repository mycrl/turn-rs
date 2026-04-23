use std::{collections::HashMap, env, time::Duration};

use anyhow::{Result, anyhow};
use tokio::{process::Command, time::sleep};
use turn_server::{
    config::{Auth, Config, Interface, Log, Server},
    service::Transport,
    start_server,
};

async fn run_coturn_uclient(transport: Transport, send_indication: bool) -> Result<()> {
    let mut args = [
        "-L",
        "127.0.0.1",
        "-e",
        "127.0.0.1",
        "-u",
        "static_credentials",
        "-w",
        "static_credentials",
        "-X",
        "-y",
        "-t",
    ]
    .to_vec();

    {
        if transport == Transport::Tcp {
            args.push("-t");
        }

        if send_indication {
            args.push("-s");
        }

        args.push("127.0.0.1");
    }

    let output = Command::new(env::var("COTURN_UCLIENT_PATH")?)
        .args(&args)
        .output()
        .await?;

    let output = String::from_utf8(output.stdout)?;

    if output.contains("ERROR")
        || !output.contains("Total lost packets 0 (0.000000%), total send dropped 0 (0.000000%)")
    {
        return Err(anyhow!(output));
    }

    println!("> turnutils_uclient {}", args.join(" "));
    println!("{}", output);

    Ok(())
}

#[tokio::test]
async fn integration_testing_with_coturn_uclient() -> Result<()> {
    {
        tokio::spawn(async move {
            start_server(Config {
                log: Log::default(),
                server: Server {
                    realm: "test".to_string(),
                    interfaces: vec![
                        Interface::Udp {
                            external: "127.0.0.1:3478".parse().unwrap(),
                            listen: "127.0.0.1:3478".parse().unwrap(),
                            idle_timeout: 30,
                            mtu: 1500,
                        },
                        Interface::Tcp {
                            external: "127.0.0.1:3478".parse().unwrap(),
                            listen: "127.0.0.1:3478".parse().unwrap(),
                            idle_timeout: 30,
                            ssl: None,
                        },
                    ],
                    ..Default::default()
                },
                auth: Auth {
                    enable_hooks_auth: false,
                    static_auth_secret: Some("static_auth_secret".to_string()),
                    static_credentials: {
                        let mut it = HashMap::with_capacity(1);
                        it.insert(
                            "static_credentials".to_string(),
                            "static_credentials".to_string(),
                        );
                        it
                    },
                },
                ..Default::default()
            })
            .await
            .unwrap();
        });

        // Give the server some time to start
        sleep(Duration::from_secs(3)).await;
    }

    run_coturn_uclient(Transport::Udp, false).await?;
    run_coturn_uclient(Transport::Udp, true).await?;
    run_coturn_uclient(Transport::Tcp, false).await?;
    run_coturn_uclient(Transport::Tcp, true).await?;

    Ok(())
}
