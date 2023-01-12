use std::collections::{HashMap, HashSet};

use near_sdk::{
    env,
    json_types::{U128, U64},
    serde::{Deserialize, Serialize},
    AccountId,
};

#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Policy {
    /// List of roles and permissions for them in the current policy.
    pub roles: Vec<RolePermission>,
    /// Default vote policy. Used when given proposal kind doesn't have special policy.
    pub default_vote_policy: VotePolicy,
    /// Proposal bond.
    pub proposal_bond: U128,
    /// Expiration period for proposals.
    pub proposal_period: U64,
    /// Bond for claiming a bounty.
    pub bounty_bond: U128,
    /// Period in which giving up on bounty is not punished.
    pub bounty_forgiveness_period: U64,
}

impl Policy {
    pub fn update_group_members(&mut self, new_members: HashSet<AccountId>, members_role: String) {
        let index = &self
            .roles
            .iter()
            .position(|role| role.name == members_role)
            .expect("Role not found");

        let mut role_permission = self.roles.remove(*index);

        match role_permission.kind {
            RoleKind::Group(_) => role_permission.kind = RoleKind::Group(new_members),
            _ => env::panic_str("Wrong role kind"),
        }

        self.roles.push(role_permission);
    }
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RolePermission {
    /// Name of the role to display to the user.
    pub name: String,
    /// Kind of the role: defines which users this permissions apply.
    pub kind: RoleKind,
    /// Set of actions on which proposals that this role is allowed to execute.
    /// <proposal_kind>:<action>
    pub permissions: HashSet<String>,
    /// For each proposal kind, defines voting policy.
    pub vote_policy: HashMap<String, VotePolicy>,
}

/// Defines configuration of the vote.
#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct VotePolicy {
    /// Kind of weight to use for votes.
    pub weight_kind: WeightKind,
    /// Minimum number required for vote to finalize.
    /// If weight kind is TokenWeight - this is minimum number of tokens required.
    ///     This allows to avoid situation where the number of staked tokens from total supply is too small.
    /// If RoleWeight - this is minimum number of votes.
    ///     This allows to avoid situation where the role is got too small but policy kept at 1/2, for example.
    pub quorum: U128,
    /// How many votes to pass this vote.
    pub threshold: WeightOrRatio,
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum RoleKind {
    /// Matches everyone, who is not matched by other roles.
    Everyone,
    /// Member greater or equal than given balance. Can use `1` as non-zero balance.
    Member(U128),
    /// Set of accounts.
    Group(HashSet<AccountId>),
}

/// Direct weight or ratio to total weight, used for the voting policy.
#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum WeightOrRatio {
    Weight(U128),
    Ratio(u64, u64),
}

/// How the voting policy votes get weigthed.
#[derive(Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum WeightKind {
    /// Using token amounts and total delegated at the moment.
    TokenWeight,
    /// Weight of the group role. Roles that don't have scoped group are not supported.
    RoleWeight,
}
