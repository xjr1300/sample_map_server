use std::io::Write;

/// Webメルカトル投影法のEPSGコード。
pub const EPSG_WGS84: i32 = 4326;
pub const EPSG_WEB_MERCATOR: i32 = 3857;

/// 文字列が都道府県コードと見なせるか判断する。
///
/// # Arguments
///
/// * `code` - 都道府県コードと見なせるか、検証する文字列。
///
/// # Returns
///
/// 文字列が都道府県コードと見なせる場合はtrue、見なせない場合はfalse。
pub fn is_prefecture_code(code: &str) -> bool {
    ("01"..="47").contains(&code)
}

/// 既存のデータを削除して登録することをユーザーに確認する。
///
/// # Arguments
///
/// * `code` - 都道府県コード。
///
/// # Returns
///
/// ユーザーが許可した場合はtrue。許可しなかった場合はfalse。
pub fn confirm_register(code: &str) -> bool {
    println!("指定された都道府県({})のレコードが登録されています。", code);
    loop {
        print!("既存のレコードを削除して登録しますか? [y/n]: ");
        std::io::stdout().flush().unwrap();
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).ok();
        let answer: String = answer.trim().parse().ok().unwrap();
        if answer.is_empty() {
            continue;
        }
        let answer = answer.to_lowercase();
        if answer.starts_with('y') {
            return true;
        } else if answer.starts_with('n') {
            break;
        }
    }

    false
}
