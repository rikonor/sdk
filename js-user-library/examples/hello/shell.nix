let pkgs = (import ../../.. {}).pkgs; in
let sdk = pkgs.dfinity-sdk.packages; in

pkgs.mkShell {
  buildInputs = [
    sdk.rust-workspace # for dfx
    pkgs.jq # for reading config
    pkgs.nodejs-10_x
  ];
  shellHook = ''
    set -e

    pushd ../..
    npm install
    npm run bundle
    popd

    npm install
    dfx build

    # Until https://github.com/dfinity-lab/actorscript/pull/693 is merged
    echo "export default ({ IDL }) => {" > build/canisters/hello/main.js
    echo "  const Text = IDL.Text;" >> build/canisters/hello/main.js
    echo "  return new IDL.ActorInterface({" >> build/canisters/hello/main.js
    echo "    'greet': IDL.Func(IDL.Obj({'0': Text}), IDL.Obj({'0': Text}))" >> build/canisters/hello/main.js
    # echo "    'greet': IDL.Func([Text], [Text])" >> build/canisters/hello/main.js
    echo "  });" >> build/canisters/hello/main.js
    echo "};" >> build/canisters/hello/main.js

    npm run bundle

    dfx start --background
    open $(jq --raw-output '"http://\(.defaults.start.address):\(.defaults.start.port)"' dfinity.json)

    set +e

    # Clean up before we exit the shell
    trap "{ \
      killall dfx nodemanager client
      exit 255; \
    }" EXIT
  '';
}
