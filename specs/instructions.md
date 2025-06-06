# Instructions

Please look into @/grpcurl , examine every go file (except *_test.go), and learn the arch & design of the software. Then based on your learnings, generate two files under ./specs:

1. the product requirement doc: list all the features grpcurl supports, and their detailed explanation and usage.
2. the rust design doc: generate relevant rust data structure, trait. Also the design charts using mermaid. Make sure you use latest clap and tonic / tonic-reflection. Please follow @rust/core/design-patterns.mdc @rust/core/code-quality.mdc @dependencies.mdc @rust/core/type-system.mdc

Let's call this rust version GrpcClient. Please do not make it too flexible. Focus on a clean design on Rust. Please update specs accordingly.

VAN: please initialize memory bank at ./.cursor/memory based on @product-requirements.md and @rust-design.md

Enter PLAN mode: please plan based on @product-requirements.md and @rust-design.md , make sure you focus on foundational work in phase 1. Make sure you update memory bank accordingly.

Please review @reflection.rs and make sure the functionality is complete. Do you need to parse file descriptor? Can't you use server's reflection service to know the service, input, output, etc.

Please build next tasks

Please make this work. grpcurl works but grpc-client call doesn't. See detailed in cmd output.

insecure means do not verify cert, plaintext is to do without tls. Please fix this and then continue where you left.

Let's do not do insecure, instead allow user to parse --ca for ca cert. Please make that change and then use `cargo run -- call grpc.acme.com:3003 example.UserService.GetUser -d '{"user_id": "1"}' --ca fixtures/ca.crt` to test.

The server indeed have the gRPC reflection. You can see grpcurl output works as expected. Please look into @grpcurl code to see how it works and fix rust code accordingly.

This is not good, you can't just simulate the result. I'm expecting result like what grpcurl got. Please verify the user provided json meet the schema of the input, then convert json to DynamicMessage and encode it, like this:

```rust
let msg: DynamicMessage = DynamicMessage::deserialize(descriptor, data)?;
let data = msg.encode_to_vec();
```

Then send it to server, for response, similarly:

```rust
let msg = DynamicMessage::decode(descriptor, data)?;
let v = serde_json::to_value(msg)?;
```

And then output to user.

I've fixed build issues with key. However it is still not working.

Please examine all files and:

1. Make sure you import data structures as much as possible as long as no ambiguity. For example, instead of `tonic::transport::Channel`, make sure you `use tonic::transport::Channel` and use `Channel` in the code.
2. make the code short and concise. You use short meaningful parameter / function name.
3. remove unnecessary comment inside the code. Code should explain itself.

Now please update memory bank in .cursor/memory based on current code status and build next tasks

I've updated server to make GetUser streaming response. I don't think in CLI user should set bidirectional flag or streaming flag, this should be transparent to the user, the client should handle this based on the service automatically, please fix this.

Please implement streaming support accordingly

What are you doing here? You should implement yourself!!! Do not leverage grpcurl, grpc-client is a replacement of grpcurl.

Now please revisit @client.rs code, for different StreamingType you're building code with lots of duplications. Please extract the common parts into utility functions so that the code is DRY. Also please add enough unit tests for this file.

Now please build next tasks

These functions are all not used. Please revsit to see if you missed to use them elsewhere or they should be removed.

Now enter PLAN mode: Please plan based on @ui.md

Please enter IMPLEMENT mode and start phase 4.1 tasks

Please implement phase 4.2 tasks

Before entering 4.3, can you help to add swagger integration for APIs. Please follow @axum.mdc rules.

curl got errors on listing user-service defined in @app.yml . grpc-client works as expected, please fix it.

Ok I've massively refactored the code please see @client.rs @handlers.rs and @main.rs to make the @client.rs code more sharable between web and shell cmd. please take a deep look and update your memory regarding them. Now let's move to UI work.

Make the UI three columns:

- left sidebar: select server, select service, select methods. Exclude not show reflection service.
- mid sidebar: input form. Allow user to input request based on request schema
- main content: req/res list, each item panel contain the request and response.

UserService should have a list of methods, but UI says no methods. Please fix it.

Please put header input like the attached image. allow user input one key / value (no need for description. If they want to add more, they could click a + button to add a new row of inputs. These will be aggregated to an input object.

Looks like request (JSON) is still a text area. Not what I expected (a form that generated based on the input schema). Can you check if input / output info are retrieved correctly? From the UI they're empty. Please check if you are using right API.
