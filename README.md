# mdotp

a bare-minimum matrix bot which lets you query your presence through a simple API, a la [Lanyard](https://github.com/Phineas/lanyard).

## use it right now
> [!WARNING]
> not all homeservers send presence information over federation, which is necessary for mdotp to work. the mdotp
> bot will DM you once you join the room if it detects that your homeserver doesn't send presence information.
> whether or not your homeserver sends this information is entirely up to its administrators -- contact them
> if you have further questions.

all you have to do is join [#mdotp:gingershaped.computer](https://matrix.to/#/#mdotp:gingershaped.computer). you will then be able to access your presence at `https://mdotp.ginger.services/api/v1/user/<your mxid>`.

## example response
see https://spec.matrix.org/v1.16/client-server-api/#mpresence for the response schema. some keys may be missing if your homeserver does not provide them.
```json
{
  "avatar_url": "mxc://gingershaped.computer/BglINGL4pgzQu9EtCmnaOayoRAOTJOr2",
  "displayname": "Ginger",
  "currently_active": true,
  "last_active_ago": 28167,
  "presence": "online"
}
```

## errors
error responses will have a non-200 status code and the following schema:
```json
{
    "error": "<error code>",
    "message": "<a human-readable message>"
}
```
possible error types:
- `bad_request` (400): the URL or body of your request was invalid
- `not_tracked` (404): the user you requested is not in the bot's room
- `presence_unavailable` (400): the homeserver of the user you requested does not provide presence information
- `internal_error` (500): internal server error. please open an issue on this repository.

## websocket
a websocket is available at `https://mdotp.ginger.services/api/v1/user/<your mxid>/ws`, which will send TEXT messages with the same schema as the REST API as soon as you connect and whenever your presence changes from then on. the websocket endpoint will return the same HTTP errors under the same circumstances as the REST endpoint.

if you leave the bot's room, the websocket will be closed. further attempts to open it will return a `not_tracked` error.

## self-hosting
the bot reads its config from a `.env` file or environment variables. an example .env file is available in this repository. it runs its own webserver on the provided host, reverse-proxy to it with caddy or nginx or similar.
