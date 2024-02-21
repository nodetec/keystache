import { useEffect, useState } from "react";

import { SatoshiV2Icon } from "@bitcoin-design/bitcoin-icons-react/filled";
import lightningInvoiceReq from "bolt11";
import QRCode from "react-qr-code";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Label } from "../ui/label";

type Props = {
  handleOpen?: (bool: boolean) => void;
  open: boolean;
  invoice: string;
};

export default function QRCodeModal({ handleOpen, invoice, open }: Props) {
  const [amount, setAmount] = useState<number | null>(null);

  useEffect(() => {
    if (invoice) {
      const decoded = lightningInvoiceReq.decode(invoice);
      if (decoded?.satoshis) {
        setAmount(decoded.satoshis);
      }
    }
  }, [invoice]);

  return (
    <Dialog open={open} onOpenChange={handleOpen}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Pay Invoice</DialogTitle>
          <DialogDescription>
            Scan this QR code with a lightning enabled wallet to pay the
            invoice.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-y-2">
          <div className="m-auto flex w-full max-w-screen-sm flex-col items-center justify-center">
            <QRCode value={invoice} />
            <Label className=" flex items-center py-4 text-left text-base font-semibold">
              Amount:
              <span className="flex items-center text-orange-500 dark:text-orange-400">
                <SatoshiV2Icon className="h-5 w-5" />
                {Number(amount).toLocaleString()}
              </span>
            </Label>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
