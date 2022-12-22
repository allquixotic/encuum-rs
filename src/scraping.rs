use std::{error::Error, sync::Arc};

use crate::structures::*;
use headless_chrome::{Browser, Element, Tab};
use regex::Regex;
use serde_json::Value;
use std::time::Duration;

type Fallible = Result<(), Box<dyn Error>>;

fn get_attr(elt: &Element, attr: &str) -> Option<String> {
    match elt.call_js_fn(&format!("function() {{ return this.getAttribute(\"{}\"); }}", attr), vec![], true).unwrap().value {
        Some(Value::String(s)) => Some(s),
        _ => None,
    }
}

fn clear_value(elt: &Element) {
    match elt.call_js_fn(r"function () {{ this.value = ''; }}", vec![], true) {
        Ok(_) => (),
        Err(e) => println!("Error clearing text area: {}", e)
    }
}

fn go(tab: &Arc<Tab>, url: &str) -> Fallible {
    tab.navigate_to(url)?;
    tab.wait_until_navigated()?;
    return Ok(());
}

pub fn scrape(opts: ScrapeOpts) -> Result<Vec<Forum>, Box<dyn Error>> {
    let mut retval: Vec<Forum> = vec!();
    let browser = Browser::default()?;
    let tab = browser.wait_for_initial_tab()?;
    tab.set_default_timeout(Duration::from_secs(60));

    //Login
    go(&tab, &format!("{}/login", &opts.baseurl))?;
    tab.wait_for_element("[name=username]")?.type_into(&opts.username)?;
    tab.wait_for_xpath("//input[@type='password']")?.type_into(&opts.password)?;
    tab.wait_for_xpath("//input[@type='submit' and @value='Login']")?.click()?;
    tab.wait_until_navigated()?;

    //Get the list of forums
    go(&tab, &format!("{}{}", &opts.baseurl, &opts.forumbase))?;
    for forum_link in tab.wait_for_elements_by_xpath("//a[contains(@class,'forum-name') or contains(@class,'subforum-')]")? {
        let url = get_attr(&forum_link, "href");
        let title = forum_link.get_inner_text();
        if url.is_none() || title.is_err() {
            println!("ERROR: Forum element has no href and/or title! Skipping it.");
            continue;
        }
        let mut f = Forum::default();
        f.base.url = url.unwrap(); //We handle None above already
        f.base.title = title.unwrap(); //We handle Err above already
        retval.push(f);
    }

    //Go through each forum and extract all the thread links
    let post_contents_rx = Regex::new(r"(?s)^\s*\[quote=@[0-9]+\]\s*(.*)\s*\[/quote\]\s*$").unwrap();
    for forum in retval.iter_mut() {

        //Go to the forum's first page of the list of threads
        go(&tab, &forum.base.url)?;

        let mut paged_loop = |extractor: &dyn Fn(u32, &mut Forum) -> Fallible| -> Fallible {
            let mut next_arrow: Option<Element> = None;
            let mut page_num: u32 = 1;

            loop {
                //Go to the next page of the thing
                if next_arrow.is_some() {
                    next_arrow.unwrap().click()?;
                    tab.wait_until_navigated()?;
                }

                //Extract stuff from the paged thing (either a forum summary or a thread)
                extractor(page_num, &mut forum)?;

                //Loop Guard - stop when there are no more pages of the forum/thread.
                next_arrow = match tab.wait_for_xpath_with_custom_timeout("//input[@class='right']", Duration::from_secs(30)) {
                    Ok(v) => Some(v),
                    Err(_) => None,
                };
                if next_arrow.is_none() {
                    println!("NOTE: Reached the last page (#{}) of forum/thread {}", page_num, forum.base.title);
                    break;
                }
                page_num += 1;
            }
            
            Ok(())
        };

        paged_loop(&|page_num, forump| {
            //Go through the list of forum threads visible *on this page* and extract all links and titles
            match tab.wait_for_elements_by_xpath("//a[contains(@class,'thread-subject')]") {
                Ok(thread_links) => 
                for thread_link in thread_links {
                    let mut ft = ForumThread::default();
                    match thread_link.get_inner_text() {
                        Ok(txt) => ft.base.title = txt,
                        Err(e) => println!("WARN: Forum thread element has no text. Error: {}", e)
                    }
                    match get_attr(&thread_link, "href") {
                        Some(href) => ft.base.url = href,
                        None => {
                            println!("WARN: Forum thread {} has no href attribute", ft.base.title)
                        }
                    }
                    forump.threads.push(ft);
                    
                },
                Err(e) => println!("NOTE: Forum {} has no thread links I can find on summary page {}. Error: {}", &forump.base.title, page_num, e)
            }
            Ok(())
        })?;

        //Depth first -- extract all forum posts on the current forum
        for thread in forum.threads.iter_mut() {

            //Open a thread
            go(&tab, &thread.base.url)?;

            let usernames = tab.wait_for_elements_by_xpath("//a[contains(@class, 'element_username')]")?; //These are only on the first page of the thread
            if usernames.len() == 0 {
                thread.poster_name = "UNKNOWN POSTER".to_string();
            }
            else {
                thread.poster_name = usernames.first().unwrap().get_inner_text()?;
            }

            //Extract all the replies to the thread.
            let mut post: Arc<Option<Post>> = Arc::new(None);
            paged_loop(&|page_num, forump| {

                //We have to get all the user names of the posters on this page.
                let mut user_name_strings: Vec<String> = vec!();
                for element in tab.wait_for_elements_by_xpath("//a[contains(@class, 'element_username')]")? {
                    user_name_strings.push(element.get_inner_text()?);
                }

                let quotes = tab.wait_for_elements_by_xpath("//div[contains(@class,'iconf-quote-right')]")?;

                for (pos, quote) in quotes.iter().enumerate() {
                    let mut ipost = Post::default();
                    let rslt_text_area = tab.wait_for_xpath("//textarea[@id='content']");
                    let mut text_area;
                    match rslt_text_area {
                        Ok(e) => text_area = e,
                        Err(e) => {
                            println!("WARN: Error getting text area: {}", e);
                            continue;
                        }
                    };
                    clear_value(&text_area);

                    quote.click()?;
                    tab.wait_until_navigated()?;

                    //Keep checking the value of the text area until it's not empty
                    let mut count = 0;
                    while match get_attr(&text_area, "value") {
                        Some(s) => if s.trim().len() > 0 {
                            Some(s)
                        } else {
                            None
                        },
                        None => None,
                    } == None && count < 30 {
                        std::thread::sleep(Duration::from_secs(1));
                        count += 1;
                    }

                    let value = get_attr(&text_area, "value").unwrap();
                    clear_value(&text_area);

                    let bbcode = match post_contents_rx.captures(&value) {
                        Some(matches) => matches.get(1).unwrap().as_str(),
                        None => ""
                    };

                    ipost.bbcode = bbcode.to_string();
                    ipost.url = tab.get_url();
                    post.replace(ipost);
                }
                Ok(())
            })?;

            //XXX: An extra memory copy because we're using closures? Ugh.
            match Option::as_ref(&post) {
                Some(pst) => thread.replies.push(pst.clone()),
                None => ()
            };
        }
    }

    return Ok(retval);
}
