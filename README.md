# Rust GraphQL example app

This project provides a complete example how to setup a Rust GraphQL web server using the following libraries:

- [Rocket](https://rocket.rs) (web server)
- [Diesel](http://diesel.rs) (database)
- [Juniper](https://github.com/graphql-rust/juniper) (graphql)
- [juniper-from-schema](https://github.com/davidpdrsn/juniper-from-schema) (graphql code generation)

## Running the app

Create a Postgres database named `graphql-app-example` with the following schema:

```
CREATE TABLE countries (
    id serial PRIMARY KEY,
    name text NOT NULL
);

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name text NOT NULL,
    country_id integer NOT NULL REFERENCES countries(id)
);
```

Then compile and run the app

```bash
$ cargo run
```

Then go to <http://localhost:8000/graphiql>.

## Note

This is by no means meant to demonstrate the best practices for making a web app with Rocket. Several important topics such as authentication and error handling is not addressed. It is meant to be used as a template for starting new apps.
