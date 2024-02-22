import { Menu } from "lucide-react";

export default function Header() {
  // TODO: hide when on /login
  // let location = useLocation();

  return (
    <div className="border-b px-4">
      <div className="flex h-12 items-center">
        <div className="flex w-full items-center space-x-4">
          <div className="flex w-full items-center justify-between gap-x-2">
            <Menu size={24} />
          </div>
        </div>
      </div>
    </div>
  );
}
