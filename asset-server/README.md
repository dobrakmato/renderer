asset-server
----------------

Server application that imports and compiles the assets on the fly.

![main view](https://i.imgur.com/NNWBel6.png)
![image asset details](https://i.imgur.com/TIgjwkw.png)

## Setup

### Single user mode

Start the asset server on you local machine. Then open the [asset server page](https://asset-server-ui.surge.sh/).
Everything should work out of the box.

### Shared (multi-user) mode

You can start the asset server application on server machine. You can configure the client to connect to
different `API_URL` than localhost (which is default).

Make sure to secure you asset server by firewall or by using reverse proxy with HTTP Basic Auth as the asset server
application provides no security or authentication mechanism.

Open the [asset server page](https://asset-server-ui.surge.sh/).

Then open the developer console <kbd>F12</kbd> in your web browser and set the `API_URL` variable in the `localStorage`
to point to your asset server hostname / IP.

```js
localStorage.setItem('API_URL', 'http://10.8.0.1:8000')
```

Then refresh the page and the client will connect to the new asset server.

-------

We hope that there will be a more user friendly process of setting up the asset server client in the future.

## Application usage

### Search bar

You can search by name, uuid or tag (partial matches are supported for all of the properties). The search is
case-insensitive.

You can also use special tokens in the search.

- `tag:rocks` will display all assets that are tagged with `rocks` tag
- `type:mesh` will display all `mesh` assets
- `dirty:` will display all dirty assets (that need recompilation)
