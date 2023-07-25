use tests::{
    create_turn,
    binding_request,
    base_allocate_request,
    allocate_request,
    create_permission_request,
    channel_bind_request,
    refresh_request,
    create_client,
};

#[tokio::test]
async fn integration_testing() {
    create_turn().await;
    let socket = create_client().await;
    binding_request(&socket).await;
    base_allocate_request(&socket).await;
    let port = allocate_request(&socket).await;
    create_permission_request(&socket, port).await;
    channel_bind_request(&socket, port).await;
    refresh_request(&socket).await;
}
