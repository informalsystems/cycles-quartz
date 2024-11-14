// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.13;

import "./Quartz.sol";
import "./openzeppelin/IERC20.sol";

/**
 * @title Transfers
 * @notice A token transfer application utilizing a Trusted Execution Environment (TEE) enclave for
 * encrypted state management.
 *
 * @dev This contract enables users to transfer ERC20 tokens with the following features:
 * - Unencrypted deposits and withdrawals: ERC20 transfers and `msg.sender` visibility prevent full
 *   encryption of these actions on-chain.
 * - Encrypted transfers: all transfers are encrypted within the enclave
 * - Encrypted balance: Token balances are stored encrypted in the contract
 * - Event-based update mechanism:
 *     - Each transfer, deposit, or withdrawal triggers an event that the enclave monitors.
 *     - Upon detecting an event, the enclave responds by calling update() to clear pending requests and
 *       process withdrawals.
 * - Multiple requests per block: When there are multiple transfer, deposit, or withdrawal requests in a
 *   block, they are handled collectively.
 * - Querying Capabilities: Provides rudimentary querying, where users query the enclave and it will
 *   store the encryptedBalance with the ephemeralPubkey the user provided.
 */
contract Transfers is Quartz {
    IERC20 public token;
    address public owner;

    /// @dev Struct to represent a request, with a type indicator and associated data
    /// Only certain params are used for each request type, to allow for the struct
    /// to represent all types of requests, as rust can (Vec<Request> can hold all types)
    struct Request {
        Action action;
        address user; // Used for Withdraw and Deposit
        uint256 amount; // Used for Deposit
        bytes32 ciphertext; // Used for Transfer type (encrypted data)
    }

    enum Action {
        DEPOSIT,
        WITHDRAW,
        TRANSFER
    }

    // User initiated events
    event Deposit(address indexed user, uint256 amount);
    event WithdrawRequest(address indexed user);
    event TransferRequest(address indexed sender, bytes32 ciphertext);
    event QueryRequestMessage(address indexed user, bytes ephemeralPubkey);
    event UpdateRequestMessage(uint256 indexed sequenceNum, bytes newEncryptedState, Transfers.Request[] requests);

    // Enclave initiated events
    event WithdrawResponse(address indexed user, uint256 amount);
    event EncryptedBalanceStored(address indexed user, bytes encryptedBalance);
    event StateUpdated(bytes newEncryptedState);

    // TODO - nat spec this
    mapping(address => bytes) public encryptedBalances;
    Request[] private requests;
    bytes public encryptedState;

    /**
     * @notice Initializes the Transfers contract with the Quartz configuration and token address.
     * @param _config The configuration object for Quartz
     * @param _quote The attestation quote for Quartz setup
     * @param _token The ERC20 token used
     */
    constructor(Config memory _config, bytes memory _quote, address _token) Quartz(_config, _quote) {
        token = IERC20(_token);
        owner = msg.sender;
    }

    /**
     * @notice Deposits tokens to the contract. Enclave will watch for UpdateRequestMessage(), and
     * then call update() to process the deposit.
     */
    function deposit(uint256 amount) external {
        require(token.transferFrom(msg.sender, address(this), amount), "Transfer failed");
        requests.push(Request(Action.DEPOSIT, msg.sender, amount, bytes32(0)));
        emit Deposit(msg.sender, amount);
        emit UpdateRequestMessage(sequenceNum, encryptedState, requests);
        sequenceNum++;
    }

    /**
     * @notice Requests to withdraw *all* tokens from the caller's balance. Enclave will watch for
     * UpdateRequestMessage(), and then call update() to process the withdrawal.
     */
    function withdraw() external {
        requests.push(Request(Action.WITHDRAW, msg.sender, 0, bytes32(0)));
        emit WithdrawRequest(msg.sender);
        emit UpdateRequestMessage(sequenceNum, encryptedState, requests);
        sequenceNum++;
    }

    /**
     * @notice Requests a transfer with encrypted ciphertext. Enclave will watch for
     * UpdateRequestMessage(), and then call update() to process the transfer.
     * @param ciphertext The encrypted transfer data (encrypted by the enclave pub key)
     */
    function transferRequest(bytes32 ciphertext) external {
        requests.push(Request(Action.TRANSFER, msg.sender, 0, ciphertext));
        emit TransferRequest(msg.sender, ciphertext);
        emit UpdateRequestMessage(sequenceNum, encryptedState, requests);
        sequenceNum++;
    }

    /**
     * @notice Updates the contract state with a new encrypted state, clears requests, and processes
     * withdrawals.
     * @dev Only enclave can call this function
     * @param newEncryptedState The new encrypted state to be stored.
     * @param withdrawalAddresses The list of addresses requesting withdrawals.
     * @param withdrawalAmounts The corresponding list of withdrawal amounts for each address.
     * @param quote The attestation quote for enclave verification.
     */
    function update(
        bytes memory newEncryptedState,
        address[] calldata withdrawalAddresses,
        uint256[] calldata withdrawalAmounts,
        bytes memory quote
    ) external onlyEnclave(quote) {
        require(withdrawalAddresses.length == withdrawalAmounts.length, "Mismatched withdrawals");

        // Store the new encrypted state
        encryptedState = newEncryptedState;
        emit StateUpdated(newEncryptedState);

        // Clear stored requests
        delete requests;

        // Process each withdrawal
        for (uint256 i = 0; i < withdrawalAddresses.length; i++) {
            address user = withdrawalAddresses[i];
            uint256 amount = withdrawalAmounts[i];
            require(token.transfer(user, amount), "Transfer failed");
            emit WithdrawResponse(user, amount);
        }
    }
    /**
     * @notice User calls this have their encrypted balance stored in the contract.
     * Enclave will watch for QueryRequestMessage(), and then call storeEncryptedBalance() to
     * store the balance.
     * @param ephemeralPubley The pubkey used to decrypt the stored balance
     */

    function queryEncryptedBalance(bytes memory ephemeralPubley) public {
        emit QueryRequestMessage(msg.sender, ephemeralPubley);
    }

    /**
     * @notice Stores an encrypted balance for a user, restricted to enclave calls.
     * @param user The address of the user whose balance is being stored
     * @param encryptedBalance The encrypted balance data
     * @param quote The attestation quote for enclave verification
     */
    function storeEncryptedBalance(address user, bytes memory encryptedBalance, bytes memory quote)
        external
        onlyEnclave(quote)
    {
        encryptedBalances[user] = encryptedBalance;
        emit EncryptedBalanceStored(user, encryptedBalance);
    }

    function getRequest(uint256 index) external view returns (Transfers.Request memory) {
        return requests[index];
    }

    /// @notice Returns the entire list of requests
    /// @return All requests stored in the contract
    function getAllRequests() public view returns (Request[] memory) {
        return requests;
    }
}
