import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { MySolanaProject } from "../target/types/my_solana_project";

describe("my_solana_project", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.MySolanaProject as Program<MySolanaProject>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().rpc();
    console.log("Your transaction signature", tx);
  });
});
