export interface UnsignedNostrEvent {
  id: string;
  pubkey: string;
  created_at: number;
  kind: number;
  tags: string[][];
  content: string;
}

export type RegisterResponse = {
  status: "success" | "error";
  message: string;
};

export type PayInvoiceResponse = "paid" | "failed" | "rejected";

export type PayInvoiceRequestHandler = (
  invoice: string,
) => Promise<PayInvoiceResponse> | PayInvoiceResponse;
