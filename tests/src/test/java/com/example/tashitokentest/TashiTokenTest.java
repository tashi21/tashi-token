package com.example.tashitokentest;

import java.math.BigInteger;
import java.nio.file.Path;

import org.assertj.core.api.Assertions;

import com.partisiablockchain.BlockchainAddress;
import com.partisiablockchain.language.abicodegen.TashiToken;
import com.partisiablockchain.language.junit.ContractBytes;
import com.partisiablockchain.language.junit.ContractTest;
import com.partisiablockchain.language.junit.JunitContractTest;

/** Test suite for the Voting contract. */
public final class TashiTokenTest extends JunitContractTest {

	private static final ContractBytes TASHI_TOKEN_CONTRACT_BYTES = ContractBytes.fromPaths(
			Path.of("../target/wasm32-unknown-unknown/release/tashi_token.wasm"),
			Path.of("../target/wasm32-unknown-unknown/release/tashi_token.abi"),
			Path.of("../target/wasm32-unknown-unknown/release/tashi_token_runner"));

	private BlockchainAddress owner;
	private BlockchainAddress alice;
	private BlockchainAddress bob;
	private BlockchainAddress contract;

	private static final BigInteger totalSupply = BigInteger.valueOf(21000000);

	/**
	 * Setup for all the other tests. Deploys a voting contract and instantiates
	 * accounts.
	 */
	@ContractTest
	void setUp() {
		owner = blockchain.newAccount(1);
		alice = blockchain.newAccount(2);
		bob = blockchain.newAccount(3);

		byte[] initializeRpc = TashiToken.initialize(totalSupply, "Tashi Token", "TAS", (byte) 8);
		contract = blockchain.deployContract(owner, TASHI_TOKEN_CONTRACT_BYTES, initializeRpc);
	}

	/** Owner will transfer transferAmount TAC to Alice and Bob each. */
	@ContractTest(previous = "setUp")
	public void transfer() {
		final BigInteger transferAmount = BigInteger.valueOf(15);
		byte[] transferToAliceRpc = TashiToken.transfer(alice, transferAmount);
		byte[] transferToBobRpc = TashiToken.transfer(bob, transferAmount);

		blockchain.sendAction(owner, contract, transferToAliceRpc);
		blockchain.sendAction(owner, contract, transferToBobRpc);
		TashiToken.TashiTokenState state = TashiToken.TashiTokenState
				.deserialize(blockchain.getContractState(contract));

		Assertions.assertThat(state.balances().get(owner)).isEqualTo(totalSupply
				.subtract(transferAmount)
				.subtract(transferAmount));
		Assertions.assertThat(state.balances().get(alice)).isEqualTo(transferAmount);
		Assertions.assertThat(state.balances().get(bob)).isEqualTo(transferAmount);
	}

	/**
	 * Alice first gives Bob an allowance of approvalAmount TAC. She then updates
	 * Bob's allowance relatively by asking for disapprovalAmount TAC back.
	 */
	@ContractTest(previous = "transfer")
	public void approveBob() {
		final BigInteger approvalAmount = BigInteger.valueOf(6);
		final BigInteger disapprovalAmount = BigInteger.valueOf(2);
		byte[] approveRpc = TashiToken.approve(bob, approvalAmount);
		byte[] approveRelativeRpc = TashiToken.approveRelative(bob, disapprovalAmount.negate());

		blockchain.sendAction(alice, contract, approveRpc);
		TashiToken.TashiTokenState state = TashiToken.TashiTokenState
				.deserialize(blockchain.getContractState(contract));

		Assertions.assertThat(state.allowed().get(alice).get(bob)).isEqualTo(approvalAmount);
		Assertions.assertThat(state.balances().get(alice)).isEqualTo(BigInteger.valueOf(15)
				.subtract(approvalAmount));

		blockchain.sendAction(alice, contract, approveRelativeRpc);
		state = TashiToken.TashiTokenState.deserialize(blockchain.getContractState(contract));

		Assertions.assertThat(state.allowed().get(alice).get(bob)).isEqualTo(approvalAmount
				.subtract(disapprovalAmount));
		Assertions.assertThat(state.balances().get(alice)).isEqualTo(BigInteger.valueOf(15)
				.subtract(approvalAmount)
				.add(disapprovalAmount));
	}

	/**
	 * Bob transfers transferAmount TAC to contract owner from his allowance
	 * from Alice.
	 */
	@ContractTest(previous = "approveBob")
	public void transferFrom() {
		final BigInteger transferAmount = BigInteger.valueOf(2);
		byte[] transferFromRpc = TashiToken.transferFrom(alice, owner, transferAmount);

		blockchain.sendAction(bob, contract, transferFromRpc);
		TashiToken.TashiTokenState state = TashiToken.TashiTokenState
				.deserialize(blockchain.getContractState(contract));

		Assertions.assertThat(state.allowed().get(alice).get(bob)).isEqualTo(BigInteger.valueOf(4)
				.subtract(transferAmount));
		Assertions.assertThat(state.balances().get(owner)).isEqualTo(totalSupply
				.subtract(BigInteger.valueOf(30))
				.add(transferAmount));
	}
}
