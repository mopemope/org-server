use std::fs;

fn generate_org_content(sections: usize, properties_per_section: usize) -> String {
    let mut content = String::new();
    
    // ファイルレベルのプロパティ
    content.push_str(":PROPERTIES:\n");
    content.push_str(":ID: generated-file\n");
    content.push_str(":TITLE: Generated Test File\n");
    content.push_str(":END:\n\n");
    
    // キーワード
    content.push_str("#+TITLE: Generated Performance Test File\n");
    content.push_str("#+TODO: TODO DOING | DONE\n");
    content.push_str("#+OPTIONS: ^:nil\n\n");
    
    for i in 0..sections {
        // ヘッドライン
        let status = match i % 3 {
            0 => "TODO",
            1 => "DOING", 
            _ => "DONE"
        };
        content.push_str(&format!("* {} Section {} :tag{}:work:\n", status, i + 1, i % 5));
        
        // スケジューリング
        if i % 2 == 0 {
            content.push_str(&format!("SCHEDULED: <2024-12-{:02} Mon 09:00>\n", (i % 28) + 1));
        } else {
            content.push_str(&format!("DEADLINE: <2024-12-{:02} Fri 17:00>\n", (i % 28) + 1));
        }
        
        // プロパティ
        if properties_per_section > 0 {
            content.push_str(":PROPERTIES:\n");
            content.push_str(&format!(":ID: section-{:04}\n", i));
            for j in 0..properties_per_section {
                content.push_str(&format!(":PROP_{}: value_{}_{}\n", j, i, j));
            }
            content.push_str(":END:\n");
        }
        
        // ドロワー
        if i % 3 == 0 {
            content.push_str(":LOGBOOK:\n");
            content.push_str(&format!("- State \"DOING\" from \"TODO\" [2024-01-{:02} Mon 10:00]\n", (i % 28) + 1));
            content.push_str(&format!("- State \"DONE\" from \"DOING\" [2024-01-{:02} Mon 15:00]\n", (i % 28) + 1));
            content.push_str(":END:\n");
        }
        
        // コンテンツ
        content.push_str(&format!("This is content for section {}.\n", i + 1));
        content.push_str("It contains multiple lines of text.\n");
        content.push_str(&format!("Here's a [[https://example.com/{}][link to resource {}]].\n", i, i));
        content.push_str("And some more descriptive text about this section.\n\n");
        
        // サブセクション
        if i % 10 == 0 && i > 0 {
            content.push_str(&format!("** Subsection {}.1\n", i + 1));
            content.push_str("Some subsection content with details.\n");
            content.push_str("More text in the subsection.\n\n");
            
            content.push_str(&format!("** Subsection {}.2\n", i + 1));
            content.push_str(":PROPERTIES:\n");
            content.push_str(&format!(":ID: subsection-{}-2\n", i));
            content.push_str(":END:\n");
            content.push_str("Another subsection with properties.\n\n");
        }
    }
    
    content
}

fn main() {
    // Medium file: 500 sections, 5 properties each
    let medium_content = generate_org_content(500, 5);
    fs::write("medium.org", medium_content).expect("Failed to write medium.org");
    
    // Large file: 2000 sections, 10 properties each  
    let large_content = generate_org_content(2000, 10);
    fs::write("large.org", large_content).expect("Failed to write large.org");
    
    println!("Generated test files:");
    println!("- medium.org: 500 sections");
    println!("- large.org: 2000 sections");
}
