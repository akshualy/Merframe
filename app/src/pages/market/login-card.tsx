import { useCallback, useState } from "react";
import { toast } from "sonner";
import { ErrorNote, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { api, errorMessage } from "@/lib/bridge";

export function LoginCard({ onDone }: { onDone: () => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);

  const handleSubmit = useCallback(async () => {
    setBusy(true);
    setFailure(null);
    try {
      const account = await api.marketLogin(email, password);
      toast.success(`Signed in as ${account.ingame_name}`);
      onDone();
    } catch (error) {
      const message = errorMessage(error);
      setFailure(message);
      toast.error(message);
    } finally {
      setBusy(false);
    }
  }, [email, password, onDone]);

  return (
    <Section
      title="Sign In"
      description="Your credentials go straight to warframe.market. Merframe keeps only the session token."
      className="max-w-xl"
    >
      <div className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          <Label htmlFor="email">Email</Label>
          <Input
            id="email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            autoComplete="username"
          />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="password">Password</Label>
          <Input
            id="password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
          />
        </div>
        {failure && <ErrorNote message={failure} />}
        <Button onClick={handleSubmit} disabled={busy || !email || !password}>
          Sign In
        </Button>
      </div>
    </Section>
  );
}
