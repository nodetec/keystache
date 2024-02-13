import { useState } from "react";

import { invoke } from "@tauri-apps/api/tauri";

import { zodResolver } from "@hookform/resolvers/zod";
import { Button } from "~/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormMessage,
} from "~/components/ui/form";
import { Input } from "~/components/ui/input";
import { getPublicKey, nip19 } from "nostr-tools";
import { useForm } from "react-hook-form";
import * as z from "zod";
import { useNavigate } from "react-router-dom";
import { RegisterResponse } from "~/types";
import useStore from "~/store";

const isValidNsec = (nsec: string) => {
  try {
    return nip19.decode(nsec).type === "nsec";
  } catch (e) {
    return false;
  }
};

const formSchema = z.object({
  nsec: z.string().refine(isValidNsec, {
    message: "Invalid nsec.",
  }),
});

export default function LoginPage() {
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const navigate = useNavigate();
  const { setPubkey } = useStore();

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      nsec: "",
    },
  });

  async function onSubmit(values: z.infer<typeof formSchema>) {
    setIsLoading(true);
    const { nsec } = values;
    console.log("nsec: ", nsec);

    const secretKeyUint8 = nip19.decode(nsec).data as Uint8Array;
    const publicKey = getPublicKey(secretKeyUint8);
    const npub = nip19.npubEncode(publicKey);

    const response: RegisterResponse = await invoke("register", {
      nsec: nsec,
      npub: npub,
    });

    if (response.status === "error") {
      console.error("error: ", response.message);
      setIsLoading(false);
      return;
    }
    setPubkey(publicKey);

    console.log("response: ", response);
    setIsLoading(false);
    navigate("/");
  }

  return (
    <div className="flex min-h-full  flex-1 flex-col justify-center pt-12 ">
      <div className="flex flex-col items-center justify-center sm:mx-auto sm:w-full sm:max-w-[22rem]">
        <div className="flex w-full flex-col space-y-4">
          <div className="flex w-full flex-col space-y-4 text-left">
            <h1 className="text-2xl font-semibold tracking-tight">
              Sign in to your account
            </h1>
          </div>

          <Form {...form}>
            <form
              className="flex flex-col gap-3"
              onSubmit={form.handleSubmit(onSubmit)}
            >
              <FormField
                control={form.control}
                name="nsec"
                render={({ field }) => (
                  <FormItem>
                    <FormControl>
                      <Input
                        disabled={isLoading}
                        placeholder="nsec..."
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <Button type="submit" disabled={isLoading}>
                Sign In
              </Button>
            </form>
          </Form>
        </div>
      </div>
    </div>
  );
}
