use actix_web::{dev::Service, http::StatusCode, web, App, HttpResponse, Responder};

#[test]
fn builder_test() {
    use crate::GovernorConfigBuilder;

    let mut builder = GovernorConfigBuilder::default();
    builder
        .period(crate::DEFAULT_PERIOD)
        .burst_size(crate::DEFAULT_BURST_SIZE);

    assert_eq!(GovernorConfigBuilder::default(), builder);

    let mut builder1 = builder.clone();
    builder1.per_millisecond(5000);
    let builder2 = builder.per_second(5);

    assert_eq!(&builder1, builder2);
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[actix_rt::test]
async fn test_server() {
    use crate::{Governor, GovernorConfigBuilder};
    use actix_web::test;

    let config = GovernorConfigBuilder::default()
        .per_millisecond(90)
        .burst_size(2)
        .finish()
        .unwrap();

    let app = test::init_service(
        App::new()
            .wrap(Governor::new(&config))
            .route("/", web::get().to(hello)),
    )
    .await;

    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);

    // First request
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = test::call_service(&app, req).await;
    assert_eq!(test.status(), StatusCode::OK);

    // Second request
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = test::call_service(&app, req).await;
    assert_eq!(test.status(), StatusCode::OK);

    // Third request -> Over limit, returns Error
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = app.call(req).await.unwrap_err();
    assert_eq!(
        test.as_response_error().status_code(),
        StatusCode::TOO_MANY_REQUESTS
    );

    // Replenish one element by waiting for >90ms
    let sleep_time = std::time::Duration::from_millis(100);
    std::thread::sleep(sleep_time);

    // First request after reset
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = test::call_service(&app, req).await;
    assert_eq!(test.status(), StatusCode::OK);

    // Second request after reset -> Again over limit, returns Error
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = app.call(req).await.unwrap_err();
    assert_eq!(
        test.as_response_error().status_code(),
        StatusCode::TOO_MANY_REQUESTS
    );
    let body = actix_web::body::to_bytes(test.error_response().into_body())
        .await
        .unwrap();
    assert_eq!(body, "Too many requests, retry in 0s");
}

#[actix_rt::test]
async fn test_method_filter() {
    use crate::{Governor, GovernorConfigBuilder, Method};
    use actix_web::test;

    let config = GovernorConfigBuilder::default()
        .per_millisecond(90)
        .burst_size(2)
        .methods(vec![Method::GET])
        .finish()
        .unwrap();

    let app = test::init_service(
        App::new()
            .wrap(Governor::new(&config))
            .route("/", web::get().to(hello))
            .route("/", web::post().to(hello)),
    )
    .await;

    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);

    // First request
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = test::call_service(&app, req).await;
    assert_eq!(test.status(), StatusCode::OK);

    // Second request
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = test::call_service(&app, req).await;
    assert_eq!(test.status(), StatusCode::OK);

    // Third request -> Over limit, returns Error
    let req = test::TestRequest::get()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = app.call(req).await.unwrap_err();
    assert_eq!(
        test.as_response_error().status_code(),
        StatusCode::TOO_MANY_REQUESTS
    );

    // Fourth request, now a POST request
    // This one is ignored by the ratelimit
    let req = test::TestRequest::post()
        .peer_addr(addr)
        .uri("/")
        .to_request();
    let test = test::call_service(&app, req).await;
    assert_eq!(test.status(), StatusCode::OK);
}
