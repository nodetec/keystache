import { useState } from "react";

import { getPublicKey } from "./tauriCommands";

const App = () => {
  const [publicKey, setPublicKey] = useState("");

  const loadPublicKey = async () => setPublicKey(await getPublicKey());

  return (
    <div className="container">
      <h1>Welcome to Keystache!</h1>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          loadPublicKey();
        }}
      >
        <button type="submit">Get Public Key</button>
      </form>

      <p>{publicKey}</p>
    </div>
  );
};

export default App;
