import { Navigate, useLocation } from "react-router-dom";

import { useEffect, useState } from "react";
import { getPublicKey } from "~/tauriCommands";

type Props = {
  children: React.ReactNode;
};

export default function ProtectedRoute({ children }: Props) {
  const location = useLocation();

  const [pubkey, setPubkey] = useState<string | undefined | null>(undefined);

  async function setPublicKey() {
    try {
      const response = await getPublicKey();
      if (!response) {
        setPubkey(null);
        return;
      }
      setPubkey(response);
    } catch (error) {
      setPubkey(null);
    }
  }

  useEffect(() => {
    void setPublicKey();
  }, []);

  if (pubkey === null) {
    // Redirect them to the /login page, but save the current location they were
    // trying to go to when they were redirected. This allows us to send them
    // along to that location after they login.
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  return children;
}
