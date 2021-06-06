# Communityvi Frontend

This is the frontend portion of the Communityvi project. It is one of the many access services for the synchronization
server.

## How to get started

Once you've installed dependencies with `npm install`, start a development server:

```shell
$ npm run dev

# or start the server and open the app in a new browser tab
$ npm run dev -- --open
```

Now you can just edit any file, and the browser will auto-reload if necessary.

## Building

To get a production build, run:

```shell
$ npm run build
```

> You can preview the built app with `npm run preview`. This should _not_ be used to serve your app in production.

## Tests

We use Jest in this project as the major test framework including the svelte-jester helpers for Svelte.

To run tests, execute:

```shell
$ npm run test
```

If you prefer to have them running using continuously, use:

```shell
$ npm run test:watch
```

### Configuring the tests to run against real resources

Our tests use mocks for the connection to the real WebSocket server (and other things), however, you can provide real
endpoints if you  prefer. By doing so, you will enable all tests running against real resources.

|Environment variable        |Description                            |Example               |
|----------------------------|---------------------------------------|----------------------|
|COMMUNITYVI_TEST_WS_ENDPOINT|Runs the tests against this endpoint.  |ws://localhost:8000/ws|
