impl RequestHandler {
    #[tool(description = "Get current weather for a city")]
    async fn get_weather(
        &self,
        #[tool(aggr)] params: WeatherParams
    ) -> Result<CallToolResult, Error> {
        // Check cache first
        let cache = self.cache.lock().await;
        if let Some(data) = cache.get(&params.city) {
            return Ok(CallToolResult::success(vec![
                Content::text(format!(
                    "Weather in {}: {}°C, {}, {}% humidity",
                    params.city, data.temperature, data.description, data.humidity
                ))
            ]));
        }
        drop(cache);
        
        // Fetch from API
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
            params.city, self.api_key
        );
        
        let response = reqwest::get(&url).await
            .map_err(|e| Error::internal_error("api_error", Some(serde_json::json!({"error": e.to_string()}))))?;
            
        let json: serde_json::Value = response.json().await
            .map_err(|e| Error::internal_error("parse_error", Some(serde_json::json!({"error": e.to_string()}))))?;
        
        let weather_data = WeatherData {
            temperature: json["main"]["temp"].as_f64().unwrap_or(0.0),
            description: json["weather"][0]["description"].as_str().unwrap_or("Unknown").to_string(),
            humidity: json["main"]["humidity"].as_u64().unwrap_or(0) as u8,
        };
        
        // Update cache
        self.cache.lock().await.insert(params.city.clone(), weather_data.clone());
        
        Ok(CallToolResult::success(vec![
            Content::text(format!(
                "Weather in {}: {}°C, {}, {}% humidity",
                params.city, weather_data.temperature, weather_data.description, weather_data.humidity
            ))
        ]))
    }
}
