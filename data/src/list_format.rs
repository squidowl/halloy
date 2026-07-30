pub fn join<T>(list: &[T]) -> String
where
    T: std::fmt::Display,
{
    if let Some((last_item, rest)) = list.split_last() {
        if let Some((first_item, rest)) = rest.split_first() {
            if rest.is_empty() {
                format!("{first_item} and {last_item}")
            } else {
                let mut next_items = String::new();

                for item in rest {
                    next_items.push_str(format!("{item}, ").as_str());
                }

                format!("{first_item}, {next_items}and {last_item}",)
            }
        } else {
            format!("{last_item}")
        }
    } else {
        String::new()
    }
}
