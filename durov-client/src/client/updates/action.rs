pub enum Action {
    Apply,
    Ignore,
    FillGap,
}

pub fn decide_action(local: i32, count: i32, remote: i32) -> Action {
    if remote == 0 {
        return Action::Apply;
    }
    match local + count {
        result if result == remote => Action::Apply,
        result if result > remote => Action::Ignore,
        result if result < remote => Action::FillGap,
        _ => unreachable!(),
    }
}
