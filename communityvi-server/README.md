# Communityvi Server

This is the backend portion of the Communityvi project. It provides the main "guts" i.e. the time and general state tracking and synchronization.

## How to get started

This project is written in 2021 Edition stable Rust, so you can immediately start the server once checked out:

```shell
$ cargo run
```

That's it!
Now you can either start the frontend project or poke the server using REST and websockets.

## Building

To get a production build, run:

```shell
$ cargo build --release
```

### Optional Features

#### Bundled Frontend

To satisfy our main goal to distribute only one binary in the end, the server can also bundle the frontend:

```shell
$ cargo build [--release] --features=bundle-frontend
```

#### API Docs

As we are transitioning over to more traditional Request-Repsonse communication, we're adding a RESTful API. In order to make navigating it easier, we'll try to descibe it as [OpenAPI v3 Specification](https://spec.openapis.org/oas/latest.html).
You can opt to bundle in [Swagger UI](https://swagger.io/tools/swagger-ui/) for easy access.

```shell
$ cargo build [--release] --features=api-docs
```

## Tests

To run tests, just execute:

```shell
$ cargo test
```

Keep in mind that our tests are written in a [Specification by Example](https://www.martinfowler.com/bliki/SpecificationByExample.html) style.
Each test method is an *example* that checks whether the user-observable behavior in its title actually matches reality.

