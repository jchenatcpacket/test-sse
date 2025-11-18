import requests

def stream_sse(url):
    """
    Connect to an SSE endpoint and print events as they arrive.
    """
    response = requests.get(url, stream=True, headers={'Accept': 'text/event-stream'})
    
    # Check if connection was successful
    if response.status_code != 200:
        print(f"Error: {response.status_code}")
        return
    
    # Process the stream
    for line in response.iter_lines():
        if line:
            decoded_line = line.decode('utf-8')
            print(decoded_line)

response = requests.get('http://localhost:8080/ping')
print(response.text) 

url = "http://localhost:8080/countdown/55"
stream_sse(url)
