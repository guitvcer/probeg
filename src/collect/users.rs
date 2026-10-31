//! Human accounts (service accounts start with `_` or sit below uid 500).

use crate::model::Household;
use crate::sys;

#[derive(Default)]
pub struct Users {
    pub names: Vec<String>,
    pub admins: Option<u64>,
    pub terminals: u64,
}

pub fn users() -> Users {
    let names = sys::run("dscl", &[".", "list", "/Users", "UniqueID"])
        .map(|text| parse_people(&text))
        .unwrap_or_default();
    let admins = sys::run("dscl", &[".", "read", "/Groups/admin", "GroupMembership"])
        .map(|text| count_admins(&text, &names));
    let terminals = sys::sessions()
        .iter()
        .filter(|s| s.line.starts_with("tty"))
        .count() as u64;
    Users {
        names,
        admins,
        terminals,
    }
}

pub fn fill(household: &mut Household, users: Users) {
    household.users = users.names;
    household.admins = users.admins;
    household.open_terminals = Some(users.terminals);
}

fn parse_people(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (name, uid) = (parts.next()?, parts.next()?.parse::<i64>().ok()?);
            (uid >= 500 && !name.starts_with('_')).then(|| name.to_string())
        })
        .collect()
}

fn count_admins(text: &str, people: &[String]) -> u64 {
    text.split_whitespace()
        .skip(1)
        .filter(|name| people.iter().any(|p| p == name))
        .count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_accounts_are_not_people() {
        let text = "_www 70\nroot 0\nnobody -2\nalex 501\nguest 502\n_low 600\n";
        assert_eq!(parse_people(text), ["alex", "guest"]);
    }

    #[test]
    fn root_is_not_counted_as_admin_person() {
        let people = vec!["alex".to_string(), "guest".to_string()];
        assert_eq!(count_admins("GroupMembership: root alex", &people), 1);
    }
}
