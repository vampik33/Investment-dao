#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
pub mod dao {
    use ink::storage::Mapping;
    // use openbrush::contracts::traits::psp22::*;
    use scale::{
        Decode,
        Encode,
    };

    type ProposalId = u64;

    #[derive(Encode, Decode)]
    #[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq, scale_info::TypeInfo))]
    pub enum VoteType {
        For,
        Against,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum GovernorError {
        AmountShouldNotBeZero,
        DurationError,
        QuorumNotReached,
        ProposalNotFound,
        ProposalAlreadyExecuted,
        VotePeriodEnded,
        AlreadyVoted,
        ProposalNotAccepted,
    }

    #[derive(Encode, Decode)]
    #[cfg_attr(
    feature = "std",
    derive(
    Debug,
    PartialEq,
    Eq,
    scale_info::TypeInfo,
    ink::storage::traits::StorageLayout
    )
    )]
    pub struct Proposal {
        to: AccountId,
        vote_start: u64,
        vote_end: u64,
        executed: bool,
        amount: Balance,
    }

    #[derive(Encode, Decode, Default)]
    #[cfg_attr(
    feature = "std",
    derive(
    Debug,
    PartialEq,
    Eq,
    scale_info::TypeInfo,
    ink::storage::traits::StorageLayout
    )
    )]
    pub struct ProposalVote {
        for_votes: u8,
        against_vote: u8,
    }

    #[ink(storage)]
    pub struct Governor {
        proposals: Mapping<ProposalId, Proposal>,
        proposal_votes: Mapping<Proposal, ProposalVote>,
        votes: Mapping<(ProposalId, AccountId), ()>,
        next_proposal_id: ProposalId,
        quorum: u8,
        governance_token: AccountId,
    }

    impl Governor {
        #[ink(constructor, payable)]
        pub fn new(governance_token: AccountId, quorum: u8) -> Self {
            // Question - How to fund contract on creation (we don't have contract address
            // yet)???

            Self {
                proposals: Mapping::default(),
                proposal_votes: Mapping::default(),
                votes: Mapping::default(),
                next_proposal_id: 0,
                quorum,
                governance_token,
            }
        }

        #[ink(message)]
        pub fn get_proposal(&mut self, proposal_id: ProposalId) -> Option<Proposal> {
            self.proposals.get(proposal_id)
        }

        #[ink(message)]
        pub fn next_proposal_id(&mut self) -> ProposalId {
            self.next_proposal_id
        }

        #[ink(message)]
        pub fn propose(
            &mut self,
            to: AccountId,
            amount: Balance,
            duration: u64,
        ) -> Result<(), GovernorError> {
            if amount == 0 {
                return Err(GovernorError::AmountShouldNotBeZero)
            }

            if duration == 0 {
                return Err(GovernorError::DurationError)
            }

            let proposal = Proposal {
                to,
                vote_start: self.now(),
                vote_end: self.now() + duration,
                executed: false,
                amount,
            };

            self.proposals.insert(self.next_proposal_id, &proposal);
            self.next_proposal_id += 1;

            Ok(())
        }

        #[ink(message)]
        pub fn vote(
            &mut self,
            proposal_id: ProposalId,
            vote: VoteType,
        ) -> Result<(), GovernorError> {
            match self.proposals.get(proposal_id) {
                Some(proposal) => {
                    if proposal.executed == true {
                        return Err(GovernorError::ProposalAlreadyExecuted)
                    }
                    if proposal.vote_end < self.now() {
                        return Err(GovernorError::VotePeriodEnded)
                    }

                    let caller = self.env().caller();
                    match self.votes.get((proposal_id, caller)) {
                        Some(_) => return Err(GovernorError::AlreadyVoted),
                        None => {
                            self.votes.insert((proposal_id, caller), &());

                            // This is not work -> panicked at 'not implemented: off-chain
                            // environment does not support contract invocation'

                            // let governance_token_balance =
                            // ink::env::call::build_call::<
                            //    ink::env::DefaultEnvironment,
                            //>(
                            //)
                            //.call(self.governance_token)
                            //.gas_limit(5000000000)
                            //.exec_input(
                            //    ink::env::call::ExecutionInput::new(
                            //        ink::env::call::Selector::new(ink::selector_bytes!(
                            //            "PSP22::balance_of"
                            //        )),
                            //    )
                            //    .push_arg(caller),
                            //)
                            //.returns::<()>()
                            //.try_invoke();

                            // let governance_token_total_supply =
                            //  <PSP22Ref>::total_supply(&self.governance_token);

                            // let governance_token_balance =
                            // <PSP22Ref>::balance_of(&self.governance_token, caller);

                            // For now this value is hardcoded
                            let governance_token_total_supply: Balance = 10;
                            let governance_token_balance: Balance = 10;

                            let weight: u8 = (governance_token_balance
                                / governance_token_total_supply
                                * 100)
                                .try_into()
                                .unwrap();

                            if let None = self.proposal_votes.get(&proposal) {
                                self.proposal_votes.insert(
                                    &proposal,
                                    &ProposalVote {
                                        for_votes: 0,
                                        against_vote: 0,
                                    },
                                );
                            }

                            let mut proposal_vote = self
                                .proposal_votes
                                .get(&proposal)
                                .expect("Just inserted, should exist");

                            match vote {
                                VoteType::For => {
                                    proposal_vote.for_votes += weight;
                                }
                                VoteType::Against => {
                                    proposal_vote.against_vote += weight;
                                }
                            };

                            self.proposal_votes.insert(proposal, &proposal_vote);
                        }
                    }
                }
                None => return Err(GovernorError::ProposalNotFound),
            }

            Ok(())
        }

        #[ink(message)]
        pub fn execute(&mut self, proposal_id: ProposalId) -> Result<(), GovernorError> {
            match self.proposals.get(proposal_id) {
                Some(mut proposal) => {
                    if proposal.executed == true {
                        return Err(GovernorError::ProposalAlreadyExecuted)
                    }

                    let proposal_vote_option = self.proposal_votes.get(&proposal);
                    if !proposal_vote_option.is_some() {
                        return Err(GovernorError::QuorumNotReached)
                    }

                    let proposal_vote = proposal_vote_option.unwrap();
                    if proposal_vote.for_votes + proposal_vote.against_vote < self.quorum
                    {
                        return Err(GovernorError::QuorumNotReached)
                    }
                    if proposal_vote.for_votes < proposal_vote.against_vote {
                        return Err(GovernorError::ProposalNotAccepted)
                    }
                    proposal.executed = true;
                    self.proposals.insert(proposal_id, &proposal);
                }
                None => return Err(GovernorError::ProposalNotFound),
            }

            Ok(())
        }

        // used for test
        #[ink(message)]
        pub fn now(&self) -> u64 {
            self.env().block_timestamp()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn create_contract(initial_balance: Balance) -> Governor {
            let accounts = default_accounts();
            set_sender(accounts.alice);
            set_balance(contract_id(), initial_balance);
            Governor::new(AccountId::from([0x01; 32]), 50)
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                account_id, balance,
            )
        }

        #[ink::test]
        fn propose_works() {
            let accounts = default_accounts();
            let mut governor = create_contract(1000);
            assert_eq!(
                governor.propose(accounts.django, 0, 1),
                Err(GovernorError::AmountShouldNotBeZero)
            );
            assert_eq!(
                governor.propose(accounts.django, 100, 0),
                Err(GovernorError::DurationError)
            );
            let result = governor.propose(accounts.django, 100, 1);
            assert_eq!(result, Ok(()));
            let proposal = governor.get_proposal(0).unwrap();
            let now = governor.now();
            assert_eq!(
                proposal,
                Proposal {
                    to: accounts.django,
                    amount: 100,
                    vote_start: 0,
                    vote_end: now + 1,
                    executed: false,
                }
            );
            assert_eq!(governor.next_proposal_id(), 1);
        }

        #[ink::test]
        fn quorum_not_reached() {
            let mut governor = create_contract(1000);
            let propose = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(propose, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Err(GovernorError::QuorumNotReached));
        }

        #[ink::test]
        fn proposal_not_accepted_with_vote_against() {
            let mut governor = create_contract(1000);
            let propose = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(propose, Ok(()));
            let vote = governor.vote(0, VoteType::Against);
            assert_eq!(vote, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Err(GovernorError::ProposalNotAccepted));
        }

        #[ink::test]
        fn proposal_accepted_with_vote_for() {
            let mut governor = create_contract(1000);
            let propose = governor.propose(AccountId::from([0x02; 32]), 100, 1);
            assert_eq!(propose, Ok(()));
            let vote = governor.vote(0, VoteType::For);
            assert_eq!(vote, Ok(()));
            let execute = governor.execute(0);
            assert_eq!(execute, Ok(()));
        }
    }
}
