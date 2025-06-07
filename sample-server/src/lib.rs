use crate::pb::{
    CreateUserRequest, GetUserRequest, IdentityInfo, ListUsersRequest, PaymentInfo,
    UpdateUserRequest, User,
    user_service_server::{UserService, UserServiceServer},
};
use anyhow::Result;
use std::{fs, net::SocketAddr, pin::Pin};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt as _, wrappers::ReceiverStream};
use tonic::{
    Request, Response, Status, Streaming,
    transport::{Identity, Server, ServerTlsConfig},
};
use tracing::info;

pub mod pb;

#[derive(Debug, Clone, Default)]
pub struct ExampleService;

pub async fn start_grpc_server(port: u16) -> Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(include_bytes!("../src/pb/example.bin"))
        .build_v1()
        .unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let service = ExampleService;

    info!("Starting server on {}", addr);

    let cert = fs::read_to_string("fixtures/certs/grpc.acme.com.crt")?;
    let key = fs::read_to_string("fixtures/certs/grpc.acme.com.key")?;
    let identity = Identity::from_pem(cert, key);
    Server::builder()
        .tls_config(ServerTlsConfig::new().identity(identity))?
        .add_service(reflection)
        .add_service(UserServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}

#[tonic::async_trait]
impl UserService for ExampleService {
    type GetUserStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send + Sync>>;
    /// Get user by ID
    async fn get_user(
        &self,
        request: Request<Streaming<GetUserRequest>>,
    ) -> Result<Response<Self::GetUserStream>, Status> {
        let (tx, rx) = mpsc::channel(10);

        let mut request = request.into_inner();
        tokio::spawn(async move {
            while let Some(request) = request.next().await {
                let Ok(request) = request else {
                    continue;
                };

                let data = Ok(User {
                    id: request.user_id,
                    name: "John Doe".to_string(),
                    email: "john.doe@example.com".to_string(),
                    addresses: vec![],
                    phone_numbers: vec![],
                    payment_info: Some(PaymentInfo {
                        card_number: "2580 1234 5678 9012".to_string(),
                        expiration_date: "12/2025".to_string(),
                        cvv: "123".to_string(),
                    }),
                    identity_info: Some(IdentityInfo {
                        ssn: "124-56-1234".to_string(),
                        drivers_license: "WB1234567".to_string(),
                        passport: "G123456789".to_string(),
                    }),
                });
                tx.send(data).await.unwrap();
            }
        });

        let stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(stream)))
    }

    async fn create_user(
        &self,
        request: tonic::Request<CreateUserRequest>,
    ) -> Result<tonic::Response<User>, tonic::Status> {
        info!("Creating user: {:?}", request.into_inner());
        Ok(tonic::Response::new(User {
            id: "1".to_string(),
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            addresses: vec![],
            phone_numbers: vec![],
            payment_info: Some(PaymentInfo {
                card_number: "2580 1234 5678 9012".to_string(),
                expiration_date: "12/2025".to_string(),
                cvv: "123".to_string(),
            }),
            identity_info: Some(IdentityInfo {
                ssn: "124-56-1234".to_string(),
                drivers_license: "WB1234567".to_string(),
                passport: "G123456789".to_string(),
            }),
        }))
    }

    async fn update_user(
        &self,
        request: tonic::Request<UpdateUserRequest>,
    ) -> Result<tonic::Response<User>, tonic::Status> {
        info!("Updating user: {:?}", request.into_inner());
        Ok(tonic::Response::new(User {
            id: "1".to_string(),
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            addresses: vec![],
            phone_numbers: vec![],
            payment_info: Some(PaymentInfo {
                card_number: "2580 1234 5678 9012".to_string(),
                expiration_date: "12/2025".to_string(),
                cvv: "123".to_string(),
            }),
            identity_info: Some(IdentityInfo {
                ssn: "124-56-1234".to_string(),
                drivers_license: "WB1234567".to_string(),
                passport: "G123456789".to_string(),
            }),
        }))
    }

    type ListUsersStream = Pin<Box<dyn Stream<Item = Result<User, Status>> + Send + Sync>>;

    async fn list_users(
        &self,
        request: Request<ListUsersRequest>,
    ) -> Result<Response<Self::ListUsersStream>, Status> {
        info!("Listing users: {:?}", request.into_inner());
        // make a stream of 3 users
        let users = vec![
            User {
                id: "1".to_string(),
                name: "John Doe".to_string(),
                email: "john.doe@example.com".to_string(),
                addresses: vec![],
                phone_numbers: vec![],
                payment_info: Some(PaymentInfo {
                    card_number: "2580 1234 5678 9012".to_string(),
                    expiration_date: "12/2025".to_string(),
                    cvv: "123".to_string(),
                }),
                identity_info: Some(IdentityInfo {
                    ssn: "124-56-1234".to_string(),
                    drivers_license: "WB1234567".to_string(),
                    passport: "G123456789".to_string(),
                }),
            },
            User {
                id: "2".to_string(),
                name: "Jane Doe".to_string(),
                email: "jane.doe@example.com".to_string(),
                addresses: vec![],
                phone_numbers: vec![],
                payment_info: Some(PaymentInfo {
                    card_number: "2580 1234 5678 9012".to_string(),
                    expiration_date: "12/2025".to_string(),
                    cvv: "123".to_string(),
                }),
                identity_info: Some(IdentityInfo {
                    ssn: "124-56-1234".to_string(),
                    drivers_license: "WB1234567".to_string(),
                    passport: "G123456789".to_string(),
                }),
            },
        ];

        let stream = tokio_stream::iter(users).map(Ok);
        Ok(Response::new(Box::pin(stream)))
    }
}
