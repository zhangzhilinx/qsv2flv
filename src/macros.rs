macro_rules! cond {
    ($condition: expr, $expr1: expr) => {
        if $condition {
            $expr1
        }
    };
    ($condition: expr, $expr1: expr, $expr2: expr) => {
        if $condition {
            $expr1
        } else {
            $expr2
        }
    };
    ($condition: expr, $stmt1: stmt) => {
        if $condition {
            $stmt1
        }
    };
}
