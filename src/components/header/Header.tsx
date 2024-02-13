import useStore from "~/store";
import { Menu } from "lucide-react";
import { shortNpub } from "react-nostr";
import { useLocation } from "react-router-dom";

export default function Header() {
  const { pubkey } = useStore();

  // TODO: hide when on /login
  let location = useLocation();

  return (
    <div className="border-b px-4">
      <div className="flex h-12 items-center">
        <div className="flex w-full items-center space-x-4">
          <div className="flex w-full items-center justify-between gap-x-2">
            <Menu size={24} />
          </div>
          {pubkey && <span>{shortNpub(pubkey)}</span>}
        </div>
      </div>
    </div>
  );
}
