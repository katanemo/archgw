use log::debug;

#[allow(dead_code)]
pub fn token_count(model_name: &str, text: &str) -> Result<usize, String> {
    debug!("getting token count model={}", model_name);
    //HACK: add support for tokenizing mistral and other models
    //filed issue https://github.com/katanemo/arch/issues/222

    let updated_model = match model_name.starts_with("gpt") {
        false => {
            debug!(
                "tiktoken_rs: unsupported model: {}, using gpt-4 to compute token count",
                model_name
            );

            "gpt-4"
        }
        true => model_name,
    };

    // Consideration: is it more expensive to instantiate the BPE object every time, or to contend the singleton?
    let bpe = tiktoken_rs::get_bpe_from_model(updated_model).map_err(|e| e.to_string())?;
    Ok(bpe.encode_ordinary(text).len())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_ordinary() {
        let model_name = "gpt-3.5-turbo";
        let text = "How many tokens does this sentence have?";
        assert_eq!(
            8,
            token_count(model_name, text).expect("correct tokenization")
        );
    }
}
